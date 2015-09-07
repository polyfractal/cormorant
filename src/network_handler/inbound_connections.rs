

use std::io::{Result, Error, ErrorKind};
use mio::tcp::{TcpStream};
use mioco::{MiocoHandle};
use uuid::Uuid;
use capnp::{serialize_packed};
use ::protocol::{command, ping};
use std::io::BufReader;

/// Spins a connection coroutine
/// Currently it just acts as an echo server, but in the near future
/// this will be responsible for pulling Cap'n'proto commands off the
/// line and dispatching to worker threads
pub fn accept(mioco: &mut MiocoHandle, stream: TcpStream) -> Result<()> {
    let peer_addr = stream.peer_addr().unwrap();

    debug!("Accepting connection from [{}]", peer_addr);

    let stream = mioco.wrap(stream);
    let mut buf_reader = BufReader::new(stream);

    loop {
        // Try to deserialize the command.  If there is a failure,
        // we assume it is a HUP for now
        let message = match serialize_packed::read_message(&mut buf_reader, ::capnp::message::ReaderOptions::new()) {
            Ok(m) => m,
            Err(e) => {
                error!("Error deserializing message: {}", e);
                break;
            }
        };

        // Since we only have one command right now, we can assume this is a Ping
        let command = match message.get_root::<command::Reader>() {
            Ok(p) => p,
            Err(e) => return Err(Error::new(ErrorKind::Other, format!("Error reading message: {}", e)))
        };

        debug!("Read command from [{}]", peer_addr);

        match command.which() {
            Ok(command::Ping(p)) => debug!("Got ping"),
            Ok(command::Pong(p)) => debug!("Got pong"),
            Err(e) => debug!("Got other")
        }
    }

    Ok(())
}
