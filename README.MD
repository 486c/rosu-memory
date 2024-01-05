# rosu-memory

Lightweight, cross-platform and simple osu! memory reader written in Rust

# Usage
Currently rosu-memory tries to auto-detect your osu! folder from running osu! process.
This means you can simply run it without providing additional arguments

But just in case you can always overwrite auto-detected path.
Instructions on how to do that are listed below.

**On Linux auto-detection doesn't work in cutting edge**

# How do I use counters/overlays?
1. Select any ported counter/overlay (you can see some inside `examples` folder)
2. Open OBS
3. Add to your scene `Browser Source`
4. Click on `Local File` checkbox
5. Choose `index.html` file inside counter/overlay folder
6. Done!

# Setting path to the osu! folder manually
### Windows
There are two primary methods
1. By setting environment value in windows settings (**recommended**):
	1.  Type `Edit the system environment variables` in your search bar, press enter
	2.  In opened window click `Environment variables` at the very bottom
	3.  Click `New...` under `System Variables` 
	4. In `Variable name` type OSU_PATH
	5. In `Variable value` type your path to osu! folder
	6. Click `Ok`
	7. Now you can start rosu-memory simply by double clicking `rosu-memory.exe` file!
2. By editing batch file:
	1. Open `windows_start.bat` file in your preferred text editor
	2. Change `set OSU_PATH=%HOMEDRIVE%%HOMEPATH%\AppData\Local\osu!` to `set OSU_PATH=<YOUR OSU PATH>`
	3. Start rosu-memory by opening `windows_start.bat file` (Note: `rosu-memory.exe` and `windows_start.bat` should be in the same folder)
### Linux  
1. Download binary or compile it by yourself
2. Set OSU_PATH environment variable or run rosu-memory with `--osu_path` argument

## Differences between gosumemory and tosu
1. Small binary size

	|         | rosu-memory  | gosumemory (v 1.3.8)  | tosu (v 1.3.0)       |
	|-------- | ------------ | --------------------- | -------------------  |
	| Windows | 0.9 MB       | 21 MB                 | 35 MB                |
	| Linux   | 1.2 MB       | 21.2 MB               | Does not support     |

2. Low RAM usage. Since Rust is a compiled language and doesn't have any garbage collection (as opposed to gosu and tosu), it has very low (almost minimal) RAM usage.
3. Low CPU usage
4. It's **blazingly** fast compared to other readers. We are using gradual pp calculation, which means we don't recalculate the whole beatmap each iteration, only calculating new objects that have been passed.
5. Background image appears almost instantly (if you used other readers you know that this is really big problem for them)
6. Statically linked binary, no runtime dependencies required (except win32 and glibc)
7. Can be compiled using musl libc!
8. Cross-platform
9. **Different JSON schema**. Small disadvantage, but rosu-memory outputs slightly different JSON than tosu and gosu. Please consider this when porting your counter to rosu-memory. (See examples folder :) ) 

## What is not implemented yet
1. Not all osu! data is present yet. But everything that is required to make simple counter is present :)
2. Strains graph
3. Any tournament stuff 
4. And many others :)

## JSON Schema
```
{
    "skin": "lain's skin",
    "playtime": 78624,
    "menu_mode": 0,
    "state": 5,
    "stars": 6.772433558139333,
    "stars_mods": 6.772433558139333,
    "current_stars": 6.486382070844266,
    "result_screen": {
        "username": "",
        "mods": 0,
        "mode": 0,
        "max_combo": 0,
        "score": 0,
        "hit_300": 0,
        "hit_100": 0,
        "hit_50": 0,
        "hit_geki": 0,
        "hit_katu": 0,
        "hit_miss": 0,
        "accuracy": 0
    },
    "gameplay": {
        "mods": 0,
        "username": "Guest",
        "score": 0,
        "hit_300": 0,
        "hit_100": 0,
        "hit_50": 0,
        "hit_geki": 0,
        "hit_katu": 0,
        "hit_miss": 0,
        "accuracy": 0.9141824751580849,
        "combo": 0,
        "max_combo": 0,
        "mode": 0,
        "slider_breaks": 0,
        "unstable_rate": 0,
        "passed_objects": 0,
        "grade": "B",
        "current_hp": 0,
        "current_hp_smooth": 0
    },
    "beatmap": {
        "artist": "Eternal Tears Of Sorrow",
        "title": "Another Me",
        "creator": "LMT",
        "difficulty": "Insane 1.36x (250bpm) CS4.2 AR9.6 OD9",
        "map_id": 2643167,
        "mapset_id": 1271974,
        "ar": 9.6,
        "cs": 4.2,
        "hp": 5,
        "od": 9,
        "beatmap_status": 2,
        "last_obj_time": 167310,
        "first_obj_time": 150,
        "bpm": 250,
        "paths": {
            "beatmap_full_path": "/path/to/osu/Songs/1271974 Eternal Tears Of Sorrow - Another Me/Eternal Tears Of Sorrow - Another Me (LMT) [Insane 1.36x (250bpm) CS4.2 AR9.6 OD9].osu",
            "beatmap_folder": "1271974 Eternal Tears Of Sorrow - Another Me",
            "beatmap_file": "Eternal Tears Of Sorrow - Another Me (LMT) [Insane 1.36x (250bpm) CS4.2 AR9.6 OD9].osu",
            "background_file": "night-3129908_1920.jpg",
            "background_path_full": "/path/to/osu/Songs/1271974 Eternal Tears Of Sorrow - Another Me/night-3129908_1920.jpg"
        }
    },
    "keyoverlay": {
        "k1_pressed": false,
        "k1_count": 0,
        "k2_pressed": false,
        "k2_count": 0,
        "m1_pressed": false,
        "m1_count": 0,
        "m2_pressed": false,
        "m2_count": 0
    },
    "current_bpm": 0,
    "kiai_now": false,
    "current_pp": 469.34991682892615,
    "fc_pp": 0,
    "ss_pp": 469.34991682892615,
    "menu_mods": 0,
    "mods_str": [],
    "plays": 7
}
```
### Notes
- All gameplay data is reset when leaving `Playing` state, so you don't need to do this manually in your counter
- `current_pp` changes depending on state:
	1. `SongSelect` - Shows SS pp's for currently selected map
	2. `Playing` - gradually calculates pp's based on your progress into beatmap and gameplay mods
	3. `ResultScreen` - calculates pp for score that appears on result screen
- `unstable_rate` - converts itself according to gameplay mods
- `fc_pp` - removes misses of scores when playing
- `ss_pp` - uses mods based on your current state
- `mods_str` changes depending on state:
	1. `SongSelect` - uses menu mods
	2. `Playing` - uses gameplay mods
	3. `ResultScreen` - uses result_screen mods
- `current_stars` - calculates stars gradualy based on your progress into beatmap and gameplay mods
- `playtime` changes depending on state:
	1. `SongSelect` - represents progress of mp3
	2. `Playing` - represents your progress into current beatmap



# Benchmarks (Linux)
### Static addresses reading
Around ~600 ms
### Reading loop
1. `The Sun The Moon The Stars +DT (x2 replay)`
	~190us
2. `Plasma Gun [Extreme] +HDDT`
	~90us

## Development
### Profiling
You can use [tracy](https://github.com/wolfpld/tracy) to profile `rosu-memory`. 
Just build `rosu` with `--features tracy-profile` argument and then connect to client.


# Command line arguments
```
Options:
  -o, --osu-path       <OSU_PATH>        Path to osu! folder [env: OSU_PATH=]
  -i, --interval       <INTERVAL>        Interval between updates in ms [default: 300]
  -e, --error-interval <ERROR_INTERVAL>  Amount of seconds waiting after critical error happened before running again
  -h, --help                             Print help
```
