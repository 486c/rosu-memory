mod structs;
mod network;
mod reading_loop;

use structs::{InnerValues, OutputValues, Clients};

use crate::network::{server_thread, handle_clients};

use crate::reading_loop::process_reading_loop;
use crate::structs::{
    StaticAddresses,
    Values,
};

use std::sync::{Arc, Mutex};
use std::path::PathBuf;

use clap::Parser;

use rosu_memory::
    memory::{
        process::{Process, ProcessTraits}, 
        error::ProcessError
    };

use eyre::{Report, Result};

#[derive(Parser, Debug)]
pub struct Args {
    /// Path to osu! folder
    #[arg(short, long, env)]
    osu_path: Option<PathBuf>,

    /// Interval between updates in ms
    #[clap(default_value = "300")]
    #[arg(short, long, value_parser=parse_interval)]
    interval: std::time::Duration,
}

fn parse_interval(
    arg: &str
) -> Result<std::time::Duration, std::num::ParseIntError> {
    let ms = arg.parse()?;
    Ok(std::time::Duration::from_millis(ms))
}


fn main() -> Result<()> {
    let _client = tracy_client::Client::start();

    let args = Args::parse();
    let output_values = Arc::new(Mutex::new(OutputValues::default()));
    let inner_values = InnerValues::default();

    let mut state = Values {
        addresses: StaticAddresses::default(),
        clients: Clients::default(),
        ivalues: inner_values,
        values: output_values,
    };
    
    // Spawning Hyper server
    let server_clients = state.clients.clone();
    std::thread::spawn(move || server_thread(server_clients));

    'init_loop: loop {
        let mut values = state.values.lock().unwrap();

        let p = match Process::initialize("osu!.exe") {
            Ok(p) => p,
            Err(e) => {
                println!("{:?}", Report::new(e));
                continue 'init_loop
            },
        };

        // OSU_PATH cli argument if provided should
        // overwrite auto detected path
        // else use auto detected path
        match args.osu_path {
            Some(ref v) => {
                println!("Using provided osu! folder path");
                values.osu_path = v.clone();
            },
            None => {
                println!("Using auto-detected osu! folder path");
                if let Some(ref dir) = p.executable_dir {
                    values.osu_path = dir.clone();
                } else {
                    return Err(Report::msg(
                        "Can't auto-detect osu! folder path \
                         nor any was provided through command \
                         line argument"
                    ));
                }
            },
        }
        
        // Checking if path exists
        if !values.osu_path.exists() {
            println!(
                "Provided osu path doesn't exists!\n Path: {}",
                &values.osu_path.to_str().unwrap()
            );

            continue 'init_loop
        };

        println!("init loop drop");
        drop(values);

        println!("Reading static signatures...");
        match StaticAddresses::new(&p) {
            Ok(v) => state.addresses = v,
            Err(e) => {
                match e.downcast_ref::<ProcessError>() {
                    Some(&ProcessError::ProcessNotFound) => 
                        continue 'init_loop,
                    #[cfg(target_os = "windows")]
                    Some(&ProcessError::OsError{ .. }) => 
                        continue 'init_loop,
                    Some(_) | None => {
                        println!("{:?}", e);
                        continue 'init_loop
                    },
                }
            },
        };

        println!("Starting reading loop");
        'main_loop: loop {
            if let Err(e) = process_reading_loop(
                &p,
                &mut state
            ) {
                match e.downcast_ref::<ProcessError>() {
                    Some(&ProcessError::ProcessNotFound) => 
                        continue 'init_loop,
                    #[cfg(target_os = "windows")]
                    Some(&ProcessError::OsError{ .. }) => 
                        continue 'init_loop,
                    Some(_) | None => {
                        println!("{:?}", e);
                        continue 'main_loop
                    },
                }
            }

            smol::block_on(async {
                handle_clients(state.values.clone(), state.clients.clone()).await;
            });

            std::thread::sleep(args.interval);
        }
    };
}
