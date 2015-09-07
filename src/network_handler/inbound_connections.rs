

use std::io::{Result, Error, ErrorKind};
use mio::tcp::{TcpStream};
use mioco::{MiocoHandle};
use uuid::Uuid;
use capnp::{serialize_packed};
use ::protocol::ping;
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
        let ping = match message.get_root::<ping::Reader>() {
            Ok(p) => p,
            Err(e) => return Err(Error::new(ErrorKind::Other, format!("Error reading message: {}", e)))
        };

        debug!("Read ping from [{}]", peer_addr);

        // The data arrives as an array of bytes, we need to copy
        // those out and create a UUID.
        // TODO: is there a ay to ask capnp for a slice of bytes?
        let id = {
            let mut id = [0u8; 16];
            let bytes = ping.get_id().unwrap();
            for (i, byte) in id.iter_mut().enumerate() {
                *byte = bytes.get(i as u32);
            }

            Uuid::from_bytes(&id).unwrap()
        };
        debug!("Remote server's ID: {}", id.to_hyphenated_string());
    }

    Ok(())
}
