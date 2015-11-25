
use std::net::{SocketAddr};
use std::str::FromStr;
use std::io::{Write, Read, Result, Error, ErrorKind};
use mio::tcp::{TcpSocket, TcpStream};
use mioco::{MiocoHandle, MailboxOuterEnd, EventSource};
use mioco;
use std::sync::{Arc, RwLock};
use config::Config;
use state::State;
use chrono::{UTC, NaiveDateTime, Duration};
use bincode::rustc_serialize::{encode, decode};
use bincode::SizeLimit::{Bounded};
use super::protocol::{PingCommand, Protocol};
use std::thread;

pub fn start(mioco: &mut MiocoHandle, state: &Arc<RwLock<State>>,
                    mut stream: &mut EventSource<TcpStream>) {
    debug!("state_machine()  ({:?})", thread::current().name());
    let timer_id = mioco.timer().id();

    let mut delta = 0;
    let mut last_time = UTC::now();
    loop {
        let event = mioco.select_read();

        if event.id() == timer_id {
            //ping(state, stream);
            let now = UTC::now();
            delta = now.timestamp() - last_time.timestamp();
            last_time = now;
            println!("Timer fired: {}, elapsed time: {}  (Thread: {:?})",
                now.format("%H:%M:%S%.9f"),
                delta, thread::current().name());

            mioco.timer().set_timeout(10000);
        } else {
            //process_reads(state, stream);
            println!("Read available: {}", UTC::now().format("%H:%M:%S%.9f").to_string());
        }
    }
}

fn ping(state: &Arc<RwLock<State>>, mut stream: &mut EventSource<TcpStream>) {
    // Until Chrono supports serde natively, we'll just
    // serialize the timestamp
    let t = UTC::now().timestamp();

    let v = {
        let reader = state.read().unwrap();
        reader.version.to_string()
    };

    let command = Protocol::Ping(PingCommand::new(t, v));

    debug!("Ping command will be: {:?}", command);

    let encoded = encode(&command, Bounded(128)).unwrap();
    match stream.try_write(&encoded) {
        Ok(Some(b)) => debug!("wrote {} bytes for Ping Command", b),
        Ok(None) => debug!("writable...but wrote 0 bytes"),
        Err(e) => {
            debug!("Not writable, error: {}", e);
        }
    }
}

fn process_reads(state: &Arc<RwLock<State>>, mut stream: &mut EventSource<TcpStream>) {
    debug!("process_reads()");

    let mut buffer = [0; 1024];

    match stream.try_read(&mut buffer) {
        Err(e) => {
            debug!("Error: {}", e);
            return;
        },
        Ok(Some(b)) => debug!("Read {} bytes", b),
        Ok(None) => {
            debug!("Read none, spurious?");
            return;
        }
    }

    let message: Protocol = match decode(&buffer) {
        Ok(p) => p,
        Err(e) => {
            panic!("Unable to decode buffer! [{}]", e);
        }
    };
    debug!("{:?}", message);

    match message {
        Protocol::Ping(p) => {
            debug!("Received a ping request.");
        },
        Protocol::Pong(p) => {
            debug!("Received a pong response.");
        }
    }
}
