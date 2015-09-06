
use uuid::Uuid;

pub struct State {
    pub node_id: Uuid
}

impl State {
    pub fn new() -> State {
        State {
            node_id: Uuid::new_v4()
        }
    }
}
