#![feature(custom_derive)]

extern crate clap;
extern crate toml;
extern crate rustc_serialize;
extern crate mio;
extern crate mioco;
extern crate capnp;
extern crate uuid;

#[macro_use] extern crate log;
extern crate env_logger;

mod config;
mod network_handler;
mod state;
mod util;

pub mod protocol {
    include!(concat!(env!("OUT_DIR"), "/ping_capnp.rs"));
}

use protocol::ping;

use std::sync::{Arc, RwLock};
use clap::{Arg, App};
use config::Config;
use network_handler::NetworkHandler;
use state::State;

fn main() {
    env_logger::init().unwrap();

    // Pull some optional arguments off the commandline
    let matches = App::new("cormorant")
                          .version("0.1")
                          .author("Zachary Tong <zacharyjtong@gmail.com>")
                          .about("Toy Distributed Key:Value Store")
                          .arg(Arg::with_name("CONFIG")
                               .short("c")
                               .long("config")
                               .help("Path to the config.toml")
                               .takes_value(true))
                          .arg(Arg::with_name("THREADS")
                               .short("t")
                               .long("threads")
                               .help("Configures the number of threads")
                               .takes_value(true))
                          .get_matches();

    // Default to 4 threads unless specified
    let threads: usize = matches.value_of("threads").unwrap_or("4").parse().unwrap();

    // Config is located in same directory as `config.toml` unless specified
    let path: String = matches.value_of("config").unwrap_or("config.toml").parse().unwrap();

    // We place the deserialized Config into an Arc, so that we can share it between
    // multiple threads in the future.  It will be immutable and not a problem to share
    let config = Arc::new(RwLock::new(Config::parse(path)));

    let state = Arc::new(RwLock::new(State::new()));

    {
        info!("Starting server \"{}\" [{}]",
            &config.read().unwrap().node.name, state.read().unwrap().node_id.to_hyphenated_string());
    }


    for i in 0..threads {
        // Placeholder for threaded worker coroutines
    }

    // The networking will be handled by the "main" thread.
    // It will be responsible for accepting connections, finding and connecting to
    // other nodes, answering requests, etc
    let (config_clone, state_clone) = (config.clone(), state.clone());
    mioco::start(move |mioco| {
        NetworkHandler::start(mioco, config_clone, state_clone)
    });
}
