
mod state_machine;
mod protocol;

use std::net::{SocketAddr, SocketAddrV4};
use std::str::FromStr;
use std::io::{Read, Write, Result};
use mio::tcp::{TcpSocket};
use mioco::{MiocoHandle};
use std::sync::{Arc, RwLock};
use config::Config;
use state::State;
use std::thread;

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

    debug!("Listening...  ({:?})", thread::current().name());
    let listener = try!(sock.listen(1024));

    // If we had to choose a new port, obtain a write-lock and update the config
    if addr.port() != bind_port {
        debug!("Updating config to port: {}", addr.port());
        let mut c = config.write().unwrap();
        c.discovery.bind_port = addr.port();
        debug!("Config updated");
    }

    info!("Server bound on {:?}  ({:?})", listener.local_addr().unwrap(), thread::current().name());

    // Spin up the discovery connections
    start_discovery(mioco, &config, &state);

    // To allow mioco to block coroutines without blocking the thread,
    // we have to wrap all mio constructs first
    let listener = mioco.wrap(listener);

    // Accept connections forever
    loop {
        let stream = try!(listener.accept());
        let state_clone = state.clone();
        debug!("Inbound connection, spawning coroutine  {:?}", thread::current().name());
        // If we get one, spawn a new coroutine and go back to
        // listening for more connections
        mioco.spawn(move |mioco| {
            let peer_addr = stream.peer_addr().unwrap();

            debug!("New coroutine: Accepting connection from [{}]  ({:?})", peer_addr, thread::current().name());

            let mut stream = mioco.wrap(stream);
            state_machine::start(mioco, &state_clone, &mut stream);
            Ok(())
        });
    }
}

fn build_address(bind_host: &String, bind_port: u16) -> SocketAddr {
    let ip = FromStr::from_str(&*bind_host)
                .unwrap_or_else(|err|panic!("{}: [{}]", err, bind_host));;
    SocketAddr::V4(SocketAddrV4::new(ip, bind_port))
}



/// Begin the discovery process.  Essentially it iterates over the list of
/// discovery nodes and spawns a coroutine to talk to each one
fn start_discovery(mioco: &mut MiocoHandle, config: &Arc<RwLock<Config>>, state: &Arc<RwLock<State>>) {

    // We need to avoid connecting to ourself, so read-lock the config
    // and format bind_host appropriately.  We could do this below, but by
    // separating it out we give other threads a chance to jump in with a write-lock
    let bind_host = {
        let reader = config.read().unwrap();
        format!("{}:{}", &reader.discovery.bind_host, &reader.discovery.bind_port)
    };

    // Try to connect to external servers, but filter out ourself
    // Obtains a read-lock on the config for the duration of the iterator
    {
        let reader = config.read().unwrap();
        for host in reader.discovery.hosts.iter().filter(|&x| x != &*bind_host) {
            let addr: SocketAddr = FromStr::from_str(&*host).unwrap_or_else(|err|panic!("{}: [{}]", err, host));
            let state_clone = state.clone();
            debug!("Connecting to external node [{}]...", addr);
            mioco.spawn(move |mioco| {
                discover(mioco, addr, state_clone)
            });
        }
    }
}

/// This function is in charge of talking to a single external node, attempting to
/// establish a connection, etc
fn discover(mioco: &mut MiocoHandle, addr: SocketAddr, state: Arc<RwLock<State>>) -> Result<()> {
    // Never gonna give you up...
    loop {

        // Create a socket and connect to our remote host
        let socket = TcpSocket::v4().unwrap();

        // Mio returns a stream and a completion flag.  If `complete` is
        // true, the socket connected immediately and we can start writing.
        // It usually returns false
        let (stream, mut complete) = socket.connect(&addr).unwrap();
        let mut stream = &mut mioco.wrap(stream);

        // If the socket didn't connect immediately, connect_wait() will
        // block this coroutine and return a Result to signal if the connection
        // completed
        if !complete {
            mioco.select_write();
            complete = match stream.with_raw(|s| s.take_socket_error()) {
                Ok(_) => true,
                Err(e) => {
                    debug!("err: {:?}", e);
                    false
                }
            }
        }

        if complete {
            state_machine::start(mioco, &state, &mut stream);
            debug!("Remote connection failed, sleep and try to reconnect...");
        }

        // Sleep for a bit, then try to re-connect
        mioco.sleep(2000);
    }
}
