# rosu-memory

Lightweight, cross-platform and simple osu! memory reader written in Rust

# Usage
### Windows
There two primary methods
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
	2. Change `set OSU_PATH=%HOMEDRIVE%%HOMEPATH%\AppData\Local\osu!` to `set OSUPATH=<YOUR OSU PATH>`
	3. Start rosu-memory by opening `windows_start.bat file` (Note: `rosu-memory.exe` and `windows_start.bat` should be in the same folder)
### Linux  
	1. Download binary or compile it by yourself
	2. Set OSU_PATH environment variable or use `--osu_path` argument
### Command line arguments
```
Options:
  -o, --osu-path <OSU_PATH>  Path to osu folder [env: OSU_PATH=]
  -i, --interval <INTERVAL>  Interval between updates in ms [default: 300]
  -h, --help                 Print help
```

## Diffrences between gosu-memory and tosu
1. Small binary size

	|         | rosu-memory  | gosu-memory (v 1.3.8) |     tosu (v 1.3.0)   |
	|-------- | ------------ | --------------------- | -------------------  |
	| Windows | 1.2 MB       | 21 MB                 | 35 MB                |
	| Linux   | 1.9 MB       | 21.2 MB               | Does not support     |

2. Small ram usage. Since Rust is compiled language and doesn't have any garbage collection (opposed to gosu and tosu) it's have very low (almost minimal) ram usage.
3. Small CPU usage
4. Statically linked binary, no runtime dependencies is required (except win32 and glibc)
5. Can be compiled using musl libc!
6. Cross-platform
7. **Diffrent JSON schema**. Little disadvantage, but rosu-memory outputs slightly diffrent JSON than tosu and gosu. Please consider this when porting your counter to rosu-memory. (See examples folder :) ) 

## What is not implemented yet
1. Not all osu data present yet. But everything that is required to make simple counter is present :)
2. Strains graph
3. Any tournament stuff 
4. Unstable rate
5. And many others :)
