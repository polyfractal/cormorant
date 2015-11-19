
// use chrono::{UTC, NaiveDateTime};
// use semver::Version;

#[derive(RustcEncodable, RustcDecodable, Debug)]
pub enum Protocol {
    Ping(PingCommand),
    Pong(PongResponse)
}

#[derive(RustcEncodable, RustcDecodable, Debug)]
pub struct PingCommand {
    pub time: i64,
    pub version: String
}

impl PingCommand {
    pub fn new(time: i64, version: String) -> PingCommand {
        PingCommand {
            time: time,
            version: version
        }
    }
}

#[derive(RustcEncodable, RustcDecodable, Debug)]
pub struct PongResponse {
    pub time: i64,
    pub version: String
}

impl PongResponse {
    pub fn new(time: i64, version: String) -> PongResponse {
        PongResponse {
            time: time,
            version: version
        }
    }
}
