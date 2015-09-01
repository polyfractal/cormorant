use std::net::SocketAddr;
use std::str::FromStr;
use std::io::{Read, Write, Result};
use mio::tcp::{TcpSocket, TcpStream};
use mioco::{MiocoHandle};
use std::sync::Arc;
use config::Config;

/// The NetworkHandler manages all the networking, connections, requests, etc
/// There technically isn't a need for a struct -- we could have used a function --
/// but I like the syntax of static struct methods :)
pub struct NetworkHandler;

impl NetworkHandler {

    /// Start spinning the networking coroutine.  This attempts to bind itself
    /// to the configured host:port, then start listening.  When connections are accepted,
    /// it will dispatch the connection to a new coroutine
    pub fn start(mioco: &mut MiocoHandle, config: Arc<Config>) -> Result<()> {

        let bind_host = &config.discovery.bind_host;
        info!("Binding {}", bind_host);

        // We'll replace all this with proper error handling in the future
        let addr: SocketAddr = FromStr::from_str(&*bind_host)
                                    .unwrap_or_else(|err|panic!("{}: [{}]", err, bind_host));
        let sock = try!(TcpSocket::v4());
        try!(sock.bind(&addr));
        let listener = try!(sock.listen(1024));

        info!("Server bound on {:?}", listener.local_addr().unwrap());

        // Spin up the discovery connections
        startDiscovery(mioco, &config);

        // To allow mioco to block coroutines without blocking the thread,
        // we have to wrap all mio constructs first
        let listener = mioco.wrap(listener);

        // Accept connections forever
        loop {
            let conn = try!(listener.accept());

            // If we get one, spawn a new coroutine and go back to
            // listening for more connections
            mioco.spawn(move |mioco| {
                connection(mioco, conn)
            });
        }
    }
}

/// Spins a connection coroutine
/// Currently it just acts as an echo server, but in the near future
/// this will be responsible for pulling Cap'n'proto commands off the
/// line and dispatching to worker threads
fn connection(mioco: &mut MiocoHandle, conn: TcpStream) -> Result<()> {
    let peer_addr = conn.peer_addr().unwrap();

    debug!("Accepting connection from [{}]", peer_addr);

    let mut conn = mioco.wrap(conn);
    let mut buf = [0u8; 1024 * 16];

    // Simple echo server loop.  Read down all the data and
    // write it back
    loop {
        let size = try!(conn.read(&mut buf));
        debug!("Read [{}] bytes from [{}]", size, peer_addr);
        if size == 0 {
            /* eof */
            debug!("HUP from [{}]", peer_addr);
            break;
        }
        debug!("Writing [{}] bytes to [{}]", size, peer_addr);
        try!(conn.write_all(&mut buf[0..size]))
    }

    Ok(())
}

fn startDiscovery(mioco: &mut MiocoHandle, config: &Arc<Config>) {
    let bind_host = &config.discovery.bind_host;

    // Try to connect to external servers, but filter out ourself
    for host in config.discovery.hosts.iter().filter(|&x| x != bind_host) {
        let addr = FromStr::from_str(&*host).unwrap_or_else(|err|panic!("{}: [{}]", err, host));

        mioco.spawn(move |mioco| {
            discovery(mioco, addr)
        });
    }

}

fn discovery(mioco: &mut MiocoHandle, addr: SocketAddr) -> Result<()> {
    // Never gonna give you up...
    loop {
        if let Ok(socket) = TcpSocket::v4() {
            if let Ok(conn) = socket.connect(&addr) {
                debug!("Connected to external node");
            }
        }
        mioco.sleep(500);
    }

}
