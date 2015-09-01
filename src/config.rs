
use std::fs::File;
use std::io::prelude::*;
use toml::{Parser, Value};
use toml;

/// Config holds all the node's static configuration, such as
/// the list of nodes to connect to, etc.  Once deserialized
/// from disk, it is distributed to all threads as an immutable structure
#[derive(RustcDecodable, Debug)]
pub struct Config  {

    /// The configurations related to networking and finding
    /// other nodes
    pub discovery: DiscoveryConfig
}

impl Config {

    /// Returns a default configuration if we don't have/find a
    /// config file
    pub fn new() -> Config {
        Config {
            discovery: DiscoveryConfig::new()
        }
    }

    /// Attempt to load and parse the config file into our Config struct.
    /// If a file cannot be found, return a default Config.
    /// If we find a file but cannot parse it, panic
    pub fn parse(path: String) -> Config {
        let mut config_toml = String::new();

        let mut file = match File::open(&path) {
            Ok(file) => file,
            Err(_)  => {
                error!("Could not find config file, using default!");
                return Config::new();
            }
        };

        file.read_to_string(&mut config_toml)
                .unwrap_or_else(|err| panic!("Error while reading config: [{}]", err));

        let mut parser = Parser::new(&config_toml);
        let toml = parser.parse();

        if toml.is_none() {
            for err in &parser.errors {
                let (loline, locol) = parser.to_linecol(err.lo);
                let (hiline, hicol) = parser.to_linecol(err.hi);
                println!("{}:{}:{}-{}:{} error: {}",
                         path, loline, locol, hiline, hicol, err.desc);
            }
            panic!("Exiting server");
        }

        let config = Value::Table(toml.unwrap());
        match toml::decode(config) {
            Some(t) => t,
            None => panic!("Error while deserializing config")
        }
    }
}

/// The configurations related to networking and finding
/// other nodes
#[derive(RustcDecodable, Debug)]
pub struct DiscoveryConfig  {
    /// The other nodes in the cluster
    pub hosts: Vec<String>,

    /// Our local address to bind to
    pub bind_host: String,

    /// The port we should listen on
    pub bind_port: u16
}

impl DiscoveryConfig {
    pub fn new() -> DiscoveryConfig {
        DiscoveryConfig {
            hosts: vec![],
            bind_host: "127.0.0.1".to_string(),
            bind_port: 19919
        }
    }
}
