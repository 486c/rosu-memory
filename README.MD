# rosu-memory

Lightweight, cross-platform and simple osu! memory reader written in Rust

# Usage
Currently rosu-memory tries to auto-detect your osu! folder from running osu! process.
This means you can simply run it without providing additional arguments

But just in case you can always overwrite auto-detected path.
Instructions on how to do that are listed below.

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
  "beatmap_full_path": "/path/to/osu/Songs/587221 Bentham - TONIGHT/Bentham - TONIGHT (dsco) [Joel].osu",
  "artist": "Bentham",
  "beatmap_folder": "587221 Bentham - TONIGHT",
  "beatmap_id": 1243461,
  "beatmap_file": "Bentham - TONIGHT (dsco) [Joel].osu",
  "background_file": "background.jpg",
  "background_path_full": "/path/to/osu/Songs/587221 Bentham - TONIGHT/background.jpg",
  "playtime": 138201,
  "menu_mode": 0,
  "status": 5,
  "stars": 7.02695812837627,
  "stars_mods": 10.418831982102235,
  "current_stars": 0,
  "ar": 9,
  "cs": 5,
  "hp": 4,
  "od": 8,
  "beatmap_status": 2,
  "username": "",
  "score": 0,
  "hit_300": 0,
  "hit_100": 0,
  "hit_50": 0,
  "hit_geki": 0,
  "hit_katu": 0,
  "hit_miss": 0,
  "accuracy": 0,
  "combo": 0,
  "max_combo": 0,
  "mode": 0,
  "slider_breaks": 0,
  "unstable_rate": 0,
  "grade": "",
  "current_hp": 0,
  "current_hp_smooth": 0,
  "bpm": 196.00028746708827,
  "current_bpm": 0,
  "kiai_now": false,
  "current_pp": 0,
  "fc_pp": 0,
  "ss_pp": 0,
  "passed_objects": 0,
  "menu_mods": 64,
  "mods": 0,
  "plays": 9,
  "last_obj_time": 225847,
  "first_obj_time": 1000
}
```
### Notes
- All gameplay data is reset when leaving `Playing` state, so you don't need to do this manually in your counter

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
