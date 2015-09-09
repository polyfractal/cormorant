

use std::io::{Result, Error, ErrorKind};
use mio::tcp::{TcpStream};
use mioco::{MiocoHandle};
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
        
    }

    Ok(())
}
