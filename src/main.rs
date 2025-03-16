mod gosu_structs;
mod network;
mod reading_loop;
mod structs;
mod utils;

use structs::{Clients, InnerValues, OutputValues};

use crate::network::{handle_clients, server_thread};

use crate::reading_loop::process_reading_loop;
use crate::structs::{State, StaticAddresses};

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use clap::Parser;

use rosu_mem::{
    error::ProcessError,
    process::{Process, ProcessTraits},
};

use eyre::{Report, Result};

// Workaround for new winello umu-run stuff
static EXCLUDE_WORDS: [&str; 2] = ["umu-run", "waitforexitandrun"];

#[derive(Parser, Debug)]
pub struct Args {
    /// Path to osu! folder
    #[arg(short, long, env)]
    osu_path: Option<PathBuf>,

    /// Interval between updates in ms
    #[clap(default_value = "300")]
    #[arg(short, long, value_parser=parse_interval_ms)]
    interval: std::time::Duration,

    /// Amount of seconds waiting after critical error happened
    /// before running again
    #[clap(default_value = "3")]
    #[arg(short, long, value_parser=parse_interval_secs)]
    error_interval: std::time::Duration,
}

fn parse_interval_ms(arg: &str) -> Result<std::time::Duration, std::num::ParseIntError> {
    let ms = arg.parse()?;
    Ok(std::time::Duration::from_millis(ms))
}

fn parse_interval_secs(arg: &str) -> Result<std::time::Duration, std::num::ParseIntError> {
    let secs = arg.parse()?;
    Ok(std::time::Duration::from_secs(secs))
}

fn main() -> Result<()> {
    let _client = tracy_client::Client::start();

    let args = Args::parse();
    let output_values = Arc::new(Mutex::new(OutputValues::default()));
    let inner_values = InnerValues::default();

    let mut state = State {
        addresses: StaticAddresses::default(),
        clients: Clients::default(),
        ivalues: inner_values,
        values: output_values,
    };

    // Spawning Hyper server
    let server_clients = state.clients.clone();
    let server_values = state.values.clone();
    std::thread::spawn(move || server_thread(server_clients, server_values));

    println!("Spawned server!");

    if args.interval != Duration::from_millis(300) {
        println!("Using non default interval: {}", args.interval.as_millis());
    }

    'init_loop: loop {
        let p = match Process::initialize("osu!.exe", &EXCLUDE_WORDS) {
            Ok(p) => {
                println!("Found process, pid - {}", p.pid);
                p
            }
            Err(e) => {
                println!("{:?}", Report::new(e));
                thread::sleep(args.error_interval);
                continue 'init_loop;
            }
        };

        let mut values = state.values.lock().unwrap();
        // OSU_PATH cli argument if provided should
        // overwrite auto detected path
        // else use auto detected path
        match args.osu_path {
            Some(ref v) => {
                println!("Using provided osu! folder path");
                values.osu_path.clone_from(v);
            }
            None => {
                println!("Using auto-detected osu! folder path");
                if let Some(ref dir) = p.executable_dir {
                    values.osu_path.clone_from(dir);
                } else {
                    return Err(Report::msg(
                        "Can't auto-detect osu! folder path \
                         nor any was provided through command \
                         line argument",
                    ));
                }
            }
        }

        // Checking if path exists
        if !values.osu_path.exists() {
            println!(
                "Provided osu path doesn't exists!\n Path: {}",
                &values.osu_path.to_str().unwrap()
            );

            return Err(Report::msg(
                "Can't auto-detect osu! folder path \
                 nor any was provided through command \
                 line argument",
            ));
        };

        drop(values);

        println!("Reading static signatures...");
        match StaticAddresses::new(&p) {
            Ok(v) => state.addresses = v,
            Err(e) => match e.downcast_ref::<ProcessError>() {
                Some(&ProcessError::ProcessNotFound) => {
                    thread::sleep(args.error_interval);
                    continue 'init_loop;
                }
                #[cfg(target_os = "windows")]
                Some(&ProcessError::OsError { .. }) => {
                    println!("{:?}", e);
                    thread::sleep(args.error_interval);
                    continue 'init_loop;
                }
                Some(_) | None => {
                    println!("{:?}", e);
                    thread::sleep(args.error_interval);
                    continue 'init_loop;
                }
            },
        };

        println!("Starting reading loop");
        'main_loop: loop {
            if let Err(e) = process_reading_loop(&p, &mut state) {
                match e.downcast_ref::<ProcessError>() {
                    Some(&ProcessError::ProcessNotFound) => {
                        thread::sleep(args.error_interval);
                        continue 'init_loop;
                    }
                    #[cfg(target_os = "windows")]
                    Some(&ProcessError::OsError { .. }) => {
                        println!("{:?}", e);
                        thread::sleep(args.error_interval);
                        continue 'init_loop;
                    }
                    Some(_) | None => {
                        println!("{:?}", e);
                        thread::sleep(args.error_interval);
                        continue 'main_loop;
                    }
                }
            }

            smol::block_on(async {
                handle_clients(state.values.clone(), state.clients.clone()).await;
            });

            std::thread::sleep(args.interval);
        }
    }
}
