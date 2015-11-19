
use uuid::Uuid;
use semver::Version;

pub struct State {
    pub node_id: Uuid,
    pub version: Version
}

impl State {
    pub fn new() -> State {
        State {
            node_id: Uuid::new_v4(),
            version: Version {
               major: 0,
               minor: 0,
               patch: 1,
               pre: vec!(),
               build: vec!()
            }
        }
    }
}
