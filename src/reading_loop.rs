use std::mem::size_of;

use eyre::Result;
use rosu_pp::{Beatmap, GameMods};
use tracy_client::*;

use rosu_mem::process::{Process, ProcessTraits};

use crate::structs::{BeatmapStatus, GameState, OutputValues, State};

/// Here cases when key overlay is not gonna be available for reading:
/// 1. Map is not fully loaded
/// 2. If key overlay is not enabled in settings
pub fn process_key_overlay(
    p: &Process,
    values: &mut OutputValues,
    ruleset_addr: i32,
) -> Result<()> {
    let keyoverlay_ptr = p.read_i32(ruleset_addr + 0xB0)?;

    if keyoverlay_ptr == 0 {
        return Ok(());
    }

    // TODO optimize using batches?

    let keyoverlay_addr = p.read_i32(p.read_i32(keyoverlay_ptr + 0x10)? + 0x4)?;

    values.keyoverlay.k1_pressed = p.read_i8(p.read_i32(keyoverlay_addr + 0x8)? + 0x1C)? != 0;

    values.keyoverlay.k1_count = p.read_i32(p.read_i32(keyoverlay_addr + 0x8)? + 0x14)? as u32;

    values.keyoverlay.k2_pressed = p.read_i8(p.read_i32(keyoverlay_addr + 0xC)? + 0x1C)? != 0;

    values.keyoverlay.k2_count = p.read_i32(p.read_i32(keyoverlay_addr + 0xC)? + 0x14)? as u32;

    values.keyoverlay.m1_pressed = p.read_i8(p.read_i32(keyoverlay_addr + 0x10)? + 0x1C)? != 0;

    values.keyoverlay.m1_count = p.read_i32(p.read_i32(keyoverlay_addr + 0x10)? + 0x14)? as u32;

    values.keyoverlay.m2_pressed = p.read_i8(p.read_i32(keyoverlay_addr + 0x14)? + 0x1C)? != 0;

    values.keyoverlay.m2_count = p.read_i32(p.read_i32(keyoverlay_addr + 0x14)? + 0x14)? as u32;

    Ok(())
}

pub fn process_gameplay(
    p: &Process,
    state: &mut State,
    values: &mut OutputValues,
    ruleset_addr: i32,
) -> Result<()> {
    let _span = span!("Gameplay data");

    if values.prev_playtime > values.playtime {
        values.reset_gameplay(&mut state.ivalues);
        state.ivalues.reset();
    }

    values.prev_playtime = values.playtime;

    if ruleset_addr == 0 {
        return Ok(());
    };

    let gameplay_base = p.read_i32(ruleset_addr + 0x68)?;

    if gameplay_base == 0 {
        return Ok(());
    }

    let score_base = p.read_i32(gameplay_base + 0x38)?;

    let hp_base = p.read_i32(gameplay_base + 0x40)?;

    // Random value but seems to work pretty well
    // TODO sometimes playtime is >150 but game doesn't have
    // values yet unreal to debug, occurs rarely and randomly
    if values.playtime > 150 {
        values.gameplay.current_hp = p.read_f64(hp_base + 0x1C)?;
        values.gameplay.current_hp_smooth = p.read_f64(hp_base + 0x14)?;
    }

    let hit_errors_base = p.read_i32(score_base + 0x38)?;

    p.read_i32_array(hit_errors_base, &mut values.gameplay.hit_errors)?;

    values.gameplay.unstable_rate = values.gameplay.calculate_unstable_rate();

    // TODO batch
    values.gameplay.mode = p.read_i32(score_base + 0x64)?;

    // TODO batch
    values.gameplay.hit_300 = p.read_i16(score_base + 0x8a)?;
    values.gameplay.hit_100 = p.read_i16(score_base + 0x88)?;
    values.gameplay.hit_50 = p.read_i16(score_base + 0x8c)?;

    values.gameplay.username = p.read_string_with_limit_from_ptr(score_base + 0x28, 30)?;

    // TODO batch
    values.gameplay.hit_geki = p.read_i16(score_base + 0x8e)?;
    values.gameplay.hit_katu = p.read_i16(score_base + 0x90)?;
    values.gameplay.hit_miss = p.read_i16(score_base + 0x92)?;

    let passed_objects = values.gameplay.passed_objects()?;

    values.gameplay.passed_objects = passed_objects;

    values.gameplay.update_accuracy();

    values.gameplay.score = p.read_i32(score_base + 0x78)?;

    values.gameplay.combo = p.read_i16(score_base + 0x94)?;
    values.gameplay.max_combo = p.read_i16(score_base + 0x68)?;

    if values.gameplay.combo < values.prev_combo && values.gameplay.hit_miss == values.prev_hit_miss
    {
        values.gameplay.slider_breaks += 1;
    }

    values.prev_hit_miss = values.gameplay.hit_miss;

    let mods_xor_base = p.read_i32(score_base + 0x1C)?;

    let mods_raw = p.read_u64(mods_xor_base + 0x8)?;

    let mods_xor1 = mods_raw & 0xFFFFFFFF;
    let mods_xor2 = mods_raw >> 32;

    // Read key overlay
    process_key_overlay(p, values, ruleset_addr)?;

    values.gameplay.mods = (mods_xor1 ^ mods_xor2) as u32;
    values.update_readable_mods();

    // Calculate pp
    values.update_current_pp(&mut state.ivalues);
    values.update_fc_pp(&mut state.ivalues);

    values.prev_passed_objects = passed_objects;
    values.prev_combo = values.gameplay.combo;

    values.gameplay.grade = values.gameplay.get_current_grade();
    values.update_current_bpm();
    values.update_kiai();

    Ok(())
}

pub fn process_reading_loop(p: &Process, state: &mut State) -> Result<()> {
    let _span = span!("reading loop");

    let values = state.values.clone();
    let mut values = values.lock().unwrap();

    let menu_mods_ptr = p.read_i32(state.addresses.menu_mods + 0x9)?;

    let menu_mods = p.read_u32(menu_mods_ptr)?;
    values.menu_mods = menu_mods;

    let playtime_ptr = p.read_i32(state.addresses.playtime + 0x5)?;
    values.playtime = p.read_i32(playtime_ptr)?;

    let beatmap_ptr = p.read_i32(state.addresses.base - 0xC)?;
    let beatmap_addr = p.read_i32(beatmap_ptr)?;

    let status_ptr = p.read_i32(state.addresses.status - 0x4)?;

    values.state = GameState::from(p.read_u32(status_ptr)?);

    // Handle leaving `Playing` state
    if values.prev_state == GameState::Playing && values.state != GameState::Playing {
        values.reset_gameplay(&mut state.ivalues);
        state.ivalues.reset();
        values.update_stars_and_ss_pp();
    }

    if beatmap_addr == 0 {
        return Ok(());
    }

    if values.state != GameState::MultiplayerLobby {
        let mut beatmap_stats_buff = [0u8; size_of::<f32>() * 4];

        p.read(
            beatmap_addr + 0x2c,
            size_of::<f32>() * 4,
            &mut beatmap_stats_buff,
        )?;

        // Safety: unwrap here because buff is already initialized
        // and filled with zeros, the worst case scenario is
        // ar, cs, od, hp going to be zero's
        values.beatmap.ar = f32::from_le_bytes(beatmap_stats_buff[0..4].try_into().unwrap());
        values.beatmap.cs = f32::from_le_bytes(beatmap_stats_buff[4..8].try_into().unwrap());
        values.beatmap.hp = f32::from_le_bytes(beatmap_stats_buff[8..12].try_into().unwrap());
        values.beatmap.od = f32::from_le_bytes(beatmap_stats_buff[12..].try_into().unwrap());

        let plays_addr = p.read_i32(state.addresses.base - 0x33)? + 0xC;
        values.plays = p.read_i32(plays_addr)?;

        values.beatmap.artist = p.read_string_with_limit_from_ptr(beatmap_addr + 0x18, 100)?;
        values.beatmap.title = p.read_string_with_limit_from_ptr(beatmap_addr + 0x24, 150)?;
        values.beatmap.creator = p.read_string_with_limit_from_ptr(beatmap_addr + 0x7C, 30)?;
        values.beatmap.difficulty = p.read_string_with_limit_from_ptr(beatmap_addr + 0xAC, 30)?;
        values.beatmap.map_id = p.read_i32(beatmap_addr + 0xC8)?; // TODO batch
        values.beatmap.mapset_id = p.read_i32(beatmap_addr + 0xCC)?; // TODO batch
    }

    values.beatmap.beatmap_status = BeatmapStatus::from(p.read_i16(beatmap_addr + 0x12C)?);

    let mut new_map = false;

    // All time values that available everywhere
    values.chat_enabled = p.read_i8(state.addresses.chat_checker - 0x20)? != 0;

    // Skin folder
    let skin_osu_ptr = p.read_i32(state.addresses.skin + 0x7)?;
    let skin_osu_base = p.read_i32(skin_osu_ptr)?;

    let skin_name = p.read_string_with_limit_from_ptr(skin_osu_base + 0x44, 300)?;

    values.skin_folder = values.osu_path.join("Skin").join(&skin_name);
    values.skin = skin_name;

    if values.state != GameState::PreSongSelect
        && values.state != GameState::MultiplayerLobby
        && values.state != GameState::MultiplayerResultScreen
    {
        let menu_mode_addr = p.read_i32(state.addresses.base - 0x33)?;

        let beatmap_file = p.read_string_with_limit_from_ptr(beatmap_addr + 0x90, 300)?;
        let beatmap_folder = p.read_string_with_limit_from_ptr(beatmap_addr + 0x78, 300)?;
        let audio_file = p.read_string_with_limit_from_ptr(beatmap_addr + 0x64, 150)?;
        values.menu_mode = p.read_i32(menu_mode_addr)?;

        values.beatmap.paths.beatmap_full_path = values.osu_path.join("Songs/");

        values.beatmap.paths.beatmap_full_path.push(&beatmap_folder);
        values.beatmap.paths.beatmap_full_path.push(&beatmap_file);

        values.beatmap.md5 = p.read_string_with_limit_from_ptr(beatmap_addr + 0x6C, 50)?;

        // Check if beatmap changed
        if (beatmap_folder != values.beatmap.paths.beatmap_folder
            || beatmap_file != values.beatmap.paths.beatmap_file
            || values.prev_menu_mode != values.menu_mode)
            && values.beatmap.paths.beatmap_full_path.exists()
        {
            let current_beatmap = match Beatmap::from_path(&values.beatmap.paths.beatmap_full_path)
            {
                Ok(beatmap) => {
                    new_map = true;

                    /*
                    TODO bring back background
                    values.beatmap.paths.background_file.clone_from(
                        &beatmap.background.filename
                    );
                    */

                    if let Some(hobj) = beatmap.hit_objects.last() {
                        values.beatmap.last_obj_time = hobj.start_time;
                    }

                    if let Some(hobj) = beatmap.hit_objects.first() {
                        values.beatmap.first_obj_time = hobj.start_time;
                    }

                    values.beatmap.bpm = beatmap.bpm();

                    Some(beatmap)
                }
                Err(_) => {
                    println!("Failed to parse beatmap");
                    None
                }
            };

            values.current_beatmap = current_beatmap;

            values.beatmap.paths.beatmap_folder = beatmap_folder;
            values.beatmap.paths.beatmap_file = beatmap_file;
            values.beatmap.paths.audio_file = audio_file;

            values.update_min_max_bpm();
            values.update_full_paths();
            values.adjust_bpm();
        }
    }

    // store the converted map so it's not converted
    // everytime it's used for pp calc
    if new_map {
        if let Some(map) = values.current_beatmap.take() {
            let converted = map.convert(values.menu_gamemode(), &GameMods::default());
            values.current_beatmap = Some(converted?);
        }

        values.update_stars_and_ss_pp();
        values.update_current_pp(&mut state.ivalues);
    }

    let ruleset_addr = p.read_i32(p.read_i32(state.addresses.rulesets - 0xb)? + 0x4)?;

    let audio_time_ptr = p.read_i32(state.addresses.audio_time_base + 0x9)?;
    values.precise_audio_time = p.read_i32(audio_time_ptr)?;

    // If this happened there is zero sense to continue
    // reading because all the values depends on this
    // address
    if ruleset_addr == 0 {
        return Ok(());
    }

    // Process result screen
    // TODO handle situations when result screen is not ready
    if values.state == GameState::ResultScreen {
        let result_base = p.read_i32(ruleset_addr + 0x38)?;

        values.result_screen.username =
            p.read_string_with_limit_from_ptr(result_base + 0x28, 30)?;

        let mods_xor_base = p.read_i32(result_base + 0x1C)?;

        // TODO batch
        let mods_xor1 = p.read_i32(mods_xor_base + 0xC)?;
        let mods_xor2 = p.read_i32(mods_xor_base + 0x8)?;

        values.result_screen.mods = (mods_xor1 ^ mods_xor2) as u32;
        values.result_screen.mode = p.read_i32(result_base + 0x64)? as u8;
        values.result_screen.score = p.read_i32(result_base + 0x78)?;

        // TODO batch
        values.result_screen.hit_300 = p.read_i16(result_base + 0x8A)?;
        values.result_screen.hit_100 = p.read_i16(result_base + 0x88)?;
        values.result_screen.hit_50 = p.read_i16(result_base + 0x8C)?;
        values.result_screen.hit_geki = p.read_i16(result_base + 0x8E)?;
        values.result_screen.hit_katu = p.read_i16(result_base + 0x90)?;

        values.result_screen.update_accuracy();
    }

    // Process gameplay
    if values.state == GameState::Playing {
        let res = process_gameplay(p, state, &mut values, ruleset_addr);

        if let Err(e) = res {
            println!("{:?}", e);
            println!("Skipped gameplay reading, probably it's not ready yet");
        }
    }

    // Handling entering `ResultScreen` state
    if values.prev_state != GameState::ResultScreen && values.state == GameState::ResultScreen {
        if values.prev_state != GameState::Playing {
            values.update_current_pp(&mut state.ivalues);
        }

        values.update_stars_and_ss_pp();
    }

    // Handling entering `SongSelect` state
    if values.prev_state != GameState::SongSelect && values.state == GameState::SongSelect {
        // Reseting pp's from result screen
        if values.prev_state == GameState::ResultScreen {
            values.current_pp = 0.0;
        }

        values.update_current_pp(&mut state.ivalues);
        values.update_stars_and_ss_pp();
        values.adjust_bpm();
    }

    // Update stars when entering `Playing` state
    if values.prev_state != GameState::Playing && values.state == GameState::Playing {
        values.reset_gameplay(&mut state.ivalues);
        values.update_stars_and_ss_pp();
        values.adjust_bpm();
    }

    // Handle mods changes inside `SongSelect` state
    if values.state == GameState::SongSelect && values.prev_menu_mods != values.menu_mods {
        values.update_stars_and_ss_pp();
        values.update_current_pp(&mut state.ivalues);
        values.adjust_bpm();
    }

    values.prev_menu_mode = values.menu_mode;
    values.prev_menu_mods = menu_mods;
    values.prev_state = values.state;

    Ok(())
}
