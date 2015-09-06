use std::net::{SocketAddr, SocketAddrV4};
use std::str::FromStr;
use std::io::{Read, Write, Result};
use mio::tcp::{TcpSocket, TcpStream};
use mioco::{MiocoHandle, MailboxOuterEnd, EventSource};
use mioco;
use std::sync::{Arc, RwLock};
use config::Config;
use state::State;

/// The NetworkHandler manages all the networking, connections, requests, etc
/// There technically isn't a need for a struct -- we could have used a function --
/// but I like the syntax of static struct methods :)
pub struct NetworkHandler;

impl NetworkHandler {

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
                Ok(_) => break,
                Err(_) => {
                    error!("Port [{}] taken, trying higher", addr.port());
                    addr = build_address(bind_host, addr.port() + 1);
                }
            }
        }
        let listener = try!(sock.listen(1024));

        // If we had to choose a new port, obtain a write-lock and update the config
        if addr.port() != bind_port {
            let mut c = config.write().unwrap();
            c.discovery.bind_port = addr.port();
            debug!("Updating config to port: {}", addr.port());
        }

        info!("Server bound on {:?}", listener.local_addr().unwrap());

        // Spin up the discovery connections
        start_discovery(mioco, &config);

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

fn build_address(bind_host: &String, bind_port: u16) -> SocketAddr {
    let ip = FromStr::from_str(&*bind_host)
                .unwrap_or_else(|err|panic!("{}: [{}]", err, bind_host));;
    SocketAddr::V4(SocketAddrV4::new(ip, bind_port))
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

/// Begin the discovery process.  Essentially it iterates over the list of
/// discovery nodes and spawns a coroutine to talk to each one
fn start_discovery(mioco: &mut MiocoHandle, config: &Arc<RwLock<Config>>) {

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
            debug!("Connecting to external node [{}]...", addr);
            mioco.spawn(move |mioco| {
                discovery(mioco, addr)
            });
        }
    }
}

/// This function is in charge of talking to a single external node, attempting to
/// establish a connection, etc
fn discovery(mioco: &mut MiocoHandle, addr: SocketAddr) -> Result<()> {
    // Never gonna give you up...
    loop {

        // Create a socket and connect to our remote host
        let socket = TcpSocket::v4().unwrap();

        // Mio returns a stream and a completion flag.  If `complete` is
        // true, the socket connected immediately and we can start writing.
        // It usually returns false
        let (stream, mut complete) = socket.connect(&addr).unwrap();
        let stream = mioco.wrap(stream);

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

        // ... spawn a thread to handle talking to the remote node
        if complete {

            // We will use the mailboxes to notify our discovery thread if
            // the remote peer fails at some point in the future, so we can
            // can try to reconnect
            let (mail_send, mail_recv) = mioco::mailbox::<bool>();

            mioco.spawn(move |mioco| {
                remote_node_handler(mioco, stream, mail_send)
            });

            // And then block this coroutine waiting for a failure notification
            let mut mail_recv = mioco.wrap(mail_recv);
            let failed = mail_recv.recv();
            debug!("Remote connection failed, sleep and try to reconnect...");
        }

        // Sleep for a bit, then try to re-connect
        mioco.sleep(2000);
    }
}

/// This function handles the communication with a healthy, connected node.
/// If the node fails, this function will signal via a mailbox and then exit
fn remote_node_handler(mioco: &mut MiocoHandle, mut stream: EventSource<TcpStream>,
                    mail_send: MailboxOuterEnd<bool>) -> Result<()> {
    loop {
        try!(stream.write("test".as_bytes()));
        debug!("Wrote [{}] bytes to remote connection", 4);
        mioco.sleep(5000);
        break;
    }

    debug!("Shutting down remote connection...");
    mail_send.send(true);
    Ok(())
}
