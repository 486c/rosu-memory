use std::borrow::Cow;

use rosu_pp::{Beatmap, BeatmapExt};
use tracy_client::*;
use eyre::Result;

use rosu_memory::memory::process::{Process, ProcessTraits};

use crate::structs::{State, GameStatus, BeatmapStatus};

pub fn process_reading_loop(
    p: &Process,
    state: &mut State
) -> Result<()> {
    let _span = span!("reading loop");

    let values = state.values.clone();
    let mut values = values.lock().unwrap();

    let menu_mods_ptr = p.read_i32(
        state.addresses.menu_mods + 0x9
    )?;

    let menu_mods = p.read_u32(menu_mods_ptr as usize)?;
    let mods_updated = menu_mods != values.menu_mods;
    values.menu_mods = menu_mods;

    let playtime_ptr = p.read_i32(state.addresses.playtime + 0x5)?;
    values.playtime = p.read_i32(playtime_ptr as usize)?;

    let beatmap_ptr = p.read_i32(state.addresses.base - 0xC)?;
    let beatmap_addr = p.read_i32(beatmap_ptr as usize)?;

    let status_ptr = p.read_i32(state.addresses.status - 0x4)?;

    let skin_ptr = p.read_i32(state.addresses.skin + 0x4)?;
    let skin_data = p.read_i32(skin_ptr as usize)?;
    values.skin = p.read_string(skin_data as usize + 0x44)?;

    values.status = GameStatus::from(
        p.read_u32(status_ptr as usize)?
    );

    if beatmap_addr == 0 {
      return Ok(())
    }

    if values.status != GameStatus::MultiplayerLobby {
        let ar_addr = beatmap_addr + 0x2c;
        let cs_addr = ar_addr + 0x04;
        let hp_addr = cs_addr + 0x04;
        let od_addr = hp_addr + 0x04;

        values.ar = p.read_f32(ar_addr as usize)?;
        values.cs = p.read_f32(cs_addr as usize)?;
        values.hp = p.read_f32(hp_addr as usize)?;
        values.od = p.read_f32(od_addr as usize)?;

        let plays_addr = p.read_i32(state.addresses.base - 0x33)? + 0xC;
        values.plays = p.read_i32(plays_addr as usize)?;

        values.artist = p.read_string((beatmap_addr + 0x18) as usize)?;
    }

    values.beatmap_status = BeatmapStatus::from(
        p.read_i16(beatmap_addr as usize + 0x130)?
    );

    let mut new_map = false;

    if values.status != GameStatus::PreSongSelect
    && values.status != GameStatus::MultiplayerLobby 
    && values.status != GameStatus::MultiplayerResultScreen {
        let beatmap_file = p.read_string((beatmap_addr + 0x94) as usize)?;
        let beatmap_folder = p.read_string((beatmap_addr + 0x78) as usize)?;
        let background_file = p.read_string((beatmap_addr + 0x68) as usize)?;
        let menu_mode_addr = p.read_i32(state.addresses.base - 0x33)?;
        values.menu_mode = p.read_i32(menu_mode_addr as usize)?;


        if beatmap_folder != values.beatmap_folder 
        || beatmap_file != values.beatmap_file {
            let mut full_path = values.osu_path.clone();
            full_path.push("Songs");
            full_path.push(&beatmap_folder);
            full_path.push(&beatmap_file);

            if full_path.exists() {
                values.current_beatmap = match Beatmap::from_path(
                    full_path
                ) {
                    Ok(beatmap) => {
                        new_map = true;
                        Some(beatmap)
                    },
                    Err(_) => {
                        println!("Failed to parse beatmap");
                        None
                    },
                }
            }
        }

        values.beatmap_file = beatmap_file;

        values.background_path_full.clear();
        values.background_path_full.push(&beatmap_folder);
        values.background_path_full.push(&background_file);
        
        values.beatmap_folder = beatmap_folder;
        values.background_file = background_file;
    }

    if let Some(beatmap) = &values.current_beatmap {
        values.bpm = beatmap.bpm();
    }

    // store the converted map so it's not converted 
    // everytime it's used for pp calc
    if new_map {
        if let Some(map) = &values.current_beatmap {
            if let Cow::Owned(converted) = map
                .convert_mode(values.menu_gamemode()) 
            {
                values.current_beatmap = Some(converted);
            }
        }
    }
    
    values.update_stars();

    let ruleset_addr = p.read_i32(
        (p.read_i32(state.addresses.rulesets - 0xb)? + 0x4) as usize
    )?;

    if values.status == GameStatus::Playing {
        let _span = span!("Gameplay data");
        if values.prev_playtime > values.playtime {
            values.reset_gameplay();
            state.ivalues.reset();
        }

        values.prev_playtime = values.playtime;

        let gameplay_base = 
            p.read_i32((ruleset_addr + 0x68) as usize)? as usize;
        let score_base = p.read_i32(gameplay_base + 0x38)? as usize;

        let hp_base: usize = p.read_i32(gameplay_base + 0x40)? as usize;

        // Random value but seems to work pretty well
        if values.playtime > 150 {
            values.current_hp = p.read_f64(hp_base + 0x1C)?;
            values.current_hp_smooth = p.read_f64(hp_base + 0x14)?;
        }

        let hit_errors_base = (
            p.read_i32(score_base + 0x38)?
        ) as usize;

        p.read_i32_array(
            hit_errors_base,
            &mut values.hit_errors
        )?;

        values.unstable_rate = values.calculate_unstable_rate();

        values.mode = p.read_i32(score_base + 0x64)?;

        values.hit_300 = p.read_i16(score_base + 0x8a)?;
        values.hit_100 = p.read_i16(score_base + 0x88)?;
        values.hit_50 = p.read_i16(score_base + 0x8c)?;

        values.username = p.read_string(score_base + 0x28)?;

        values.hit_geki = p.read_i16(score_base + 0x8e)?;
        values.hit_katu = p.read_i16(score_base + 0x90)?;
        values.hit_miss = p.read_i16(score_base + 0x92)?;

        let passed_objects = values.passed_objects()?;
        values.passed_objects = passed_objects;

        values.accuracy = values.get_accuracy();

        values.score = p.read_i32(score_base + 0x78)?;

        values.combo = p.read_i16(score_base + 0x94)?;
        values.max_combo = p.read_i16(score_base + 0x68)?;

        if values.prev_combo > values.combo {
            values.prev_combo = 0;
        }

        if values.combo < values.prev_combo
        && values.hit_miss == values.prev_hit_miss {
            values.slider_breaks += 1;
        }

        values.prev_hit_miss = values.hit_miss;

        let mods_xor_base = (
            p.read_i32(score_base + 0x1C)?
        ) as usize;

        let mods_raw = p.read_u64(mods_xor_base + 0x8)?;

        let mods_xor1 = mods_raw & 0xFFFFFFFF;
        let mods_xor2 = mods_raw >> 32;

        values.mods = (mods_xor1 ^ mods_xor2) as u32;

        // Calculate pp
        values.update_current(&mut state.ivalues);
        values.fc_pp = values.get_fc_pp(&mut state.ivalues);

        values.prev_passed_objects = passed_objects;
        
        values.grade = values.get_current_grade();
        values.current_bpm = values.get_current_bpm();
        values.kiai_now = values.get_kiai();

        // Placing at the very end cuz we should
        // keep up with current_bpm & unstable rate
        // updates
        values.adjust_bpm();
    }

    Ok(())
}
