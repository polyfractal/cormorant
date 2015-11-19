

use std::io::{Result, Error, ErrorKind, Read, Write};
use mio::tcp::{TcpStream};
use mioco::{MiocoHandle};
use std::io::BufReader;
use bincode::rustc_serialize::{encode, decode};
use bincode::SizeLimit::{Bounded};
use std::sync::{Arc, RwLock};
use super::protocol::{Protocol, PingCommand, PongResponse};
use chrono::{UTC, NaiveDateTime};
use state::State;


/// Spins a connection coroutine
/// Currently it just acts as an echo server, but in the near future
/// this will be responsible for pulling Cap'n'proto commands off the
/// line and dispatching to worker threads
pub fn accept(mioco: &mut MiocoHandle, stream: TcpStream, state: Arc<RwLock<State>>) -> Result<()> {
    let peer_addr = stream.peer_addr().unwrap();

    debug!("Accepting connection from [{}]", peer_addr);

    let mut stream = mioco.wrap(stream);
    //let mut buf_reader = BufReader::new(stream);

    loop {
        // Try to deserialize the command.  If there is a failure,
        // we assume it is a HUP for now
        debug!("selecting...");
        //let event = mioco.select_read();
        debug!("Has read, deserializing.");

        let mut buffer = Vec::new();
        stream.read(&mut buffer);
        let message: Protocol = decode(&buffer).unwrap();
        debug!("{:?}", message);

        match message {
            Protocol::Ping(p) => {
                debug!("Received a ping.");
                let t = UTC::now().timestamp();

                let v = {
                    let reader = state.read().unwrap();
                    reader.version.to_string()
                };
                let command = Protocol::Pong(PongResponse::new(t, v));

                //mioco.select_write();
                let command = encode(&command, Bounded(128)).unwrap();
                stream.write(&command);
                debug!("Wrote pong in response to ping");
            },
            _ => debug!("Received a message we are unable to handle")
        }
    }

    Ok(())
}
