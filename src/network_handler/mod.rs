
mod outbound_connections;
mod inbound_connections;

use std::net::{SocketAddr, SocketAddrV4};
use std::str::FromStr;
use std::io::{Read, Write, Result};
use mio::tcp::{TcpSocket};
use mioco::{MiocoHandle};
use std::sync::{Arc, RwLock};
use config::Config;
use state::State;

/// Start spinning the networking coroutine.  This attempts to bind itself
/// to the configured host:port, then start listening.  When connections are accepted,
/// it will dispatch the connection to a new coroutine
pub fn start(mioco: &mut MiocoHandle, config: Arc<RwLock<Config>>, state: Arc<RwLock<State>>) -> Result<()> {

    // Obtain a read-lock, make a copy of host/port so we can release the lock
    // This is the only thread that requires that data, so it is safe to cache
    // a local copy
    let (bind_host, bind_port) = {
        let reader = config.read().unwrap();
        (&reader.discovery.bind_host.clone(), reader.discovery.bind_port.clone())
    };

    let mut addr = build_address(bind_host, bind_port);
    info!("Binding {}...", addr);

    // We'll replace all this with proper error handling in the future
    let sock = try!(TcpSocket::v4());

    // Try to bind the requested host:port, but if the port is taken,
    // keep incrementing until we find one
    loop {
        match sock.bind(&addr) {
            Ok(_) => {
                debug!("bound! on [{}]", addr);
                break;
            },
            Err(_) => {
                error!("Port [{}] taken, trying higher", addr.port());
                addr = build_address(bind_host, addr.port() + 1);
                debug!("{:?}", addr);
            }
        }
    }

    debug!("Listening...");
    let listener = try!(sock.listen(1024));

    // If we had to choose a new port, obtain a write-lock and update the config
    if addr.port() != bind_port {
        debug!("Updating config to port: {}", addr.port());
        let mut c = config.write().unwrap();
        c.discovery.bind_port = addr.port();
        debug!("Config updated");
    }

    info!("Server bound on {:?}", listener.local_addr().unwrap());

    // Spin up the discovery connections
    outbound_connections::start_discovery(mioco, &config, &state);

    // To allow mioco to block coroutines without blocking the thread,
    // we have to wrap all mio constructs first
    let listener = mioco.wrap(listener);

    // Accept connections forever
    loop {
        let conn = try!(listener.accept());

        // If we get one, spawn a new coroutine and go back to
        // listening for more connections
        mioco.spawn(move |mioco| {
            inbound_connections::accept(mioco, conn)
        });
    }
}

fn build_address(bind_host: &String, bind_port: u16) -> SocketAddr {
    let ip = FromStr::from_str(&*bind_host)
                .unwrap_or_else(|err|panic!("{}: [{}]", err, bind_host));;
    SocketAddr::V4(SocketAddrV4::new(ip, bind_port))
}
