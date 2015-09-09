
use std::net::{SocketAddr};
use std::str::FromStr;
use std::io::{Read, Result};
use mio::tcp::{TcpSocket, TcpStream};
use mioco::{MiocoHandle, MailboxOuterEnd, EventSource};
use mioco;
use std::sync::{Arc, RwLock};
use config::Config;
use state::State;
use capnp::{message, serialize_packed};
use ::protocol::{command};
use chrono::UTC;

/// Begin the discovery process.  Essentially it iterates over the list of
/// discovery nodes and spawns a coroutine to talk to each one
pub fn start_discovery(mioco: &mut MiocoHandle, config: &Arc<RwLock<Config>>, state: &Arc<RwLock<State>>) {

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

        // ... spawn a coroutine to handle talking to the remote node
        if complete {

            // We will use the mailboxes to notify our discovery thread if
            // the remote peer fails at some point in the future, so we can
            // can try to reconnect
            let (mail_send, mail_recv) = mioco::mailbox::<bool>();
            let state_clone = state.clone();
            mioco.spawn(move |mioco| {
                remote_node_handler(mioco, state_clone, stream, mail_send)
            });

            // And then block this coroutine waiting for a failure notification
            let mut mail_recv = mioco.wrap(mail_recv);
            let _ = mail_recv.recv();
            debug!("Remote connection failed, sleep and try to reconnect...");
        }

        // Sleep for a bit, then try to re-connect
        mioco.sleep(2000);
    }
}

/// This function handles the communication with a healthy, connected node.
/// If the node fails, this function will signal via a mailbox and then exit
fn remote_node_handler(mioco: &mut MiocoHandle, state: Arc<RwLock<State>>,
                    mut stream: EventSource<TcpStream>, failure: MailboxOuterEnd<bool>) -> Result<()> {
    loop {

        // Start building a Cap'n'proto ping message.  This code is
        // nested in several scopes to maintain borrowing lifetimes

        // message: capnp::message::Builder<_>
        let mut message = message::Builder::new_default();
        {
            // command: protocol::command::Builder<'_>
            let command = message.init_root::<command::Builder>();
            {
                // ping: protocol::ping::Builder<'_>
                let mut ping = command.init_ping();
                {
                    ping.set_time(UTC::now().timestamp());
                }

            }
        }

        match serialize_packed::write_message(&mut stream, &message) {
            Ok(_) => {},
            Err(e) => {
                debug!("Error while writing message: {:?}", e);
                break;
            }
        }
        debug!("Wrote ping to remote connection");
        mioco.sleep(10000);
    }

    debug!("Shutting down remote connection...");
    let _ = failure.send(true);
    Ok(())
}
