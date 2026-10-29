use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread::spawn;

use anyhow::{Error, Result};
use zmq::{Context, PAIR, Socket, SocketEvent};

static MONITOR_ENDPOINT_ID: AtomicUsize = AtomicUsize::new(0);

/// Live peer counts of a `Proxy`. `producers` counts connections on its XSUB side and `consumers`
/// counts connections accepted on its XPUB side. Both are kept up to date by socket monitor events.
#[derive(Debug, Clone, Default)]
pub struct PeerCounts {
    pub(crate) producers: Arc<AtomicUsize>,
    pub(crate) consumers: Arc<AtomicUsize>,
}

impl PeerCounts {
    pub fn producers(&self) -> usize {
        self.producers.load(Ordering::SeqCst)
    }

    pub fn consumers(&self) -> usize {
        self.consumers.load(Ordering::SeqCst)
    }
}

/// Counts the peers of `socket` on a background thread: accepted and connected peers increase the
/// count, disconnected peers decrease it. Must run before the socket connects or binds, otherwise
/// events that happen earlier are never seen.
pub(crate) fn watch_peers(
    context: &Context,
    socket: &Socket,
    count: Arc<AtomicUsize>,
) -> Result<(), Error> {
    let id = MONITOR_ENDPOINT_ID.fetch_add(1, Ordering::SeqCst);
    let endpoint = format!("inproc://peer-monitor-{}", id);
    let events = SocketEvent::ACCEPTED.to_raw()
        | SocketEvent::CONNECTED.to_raw()
        | SocketEvent::DISCONNECTED.to_raw();

    socket.monitor(&endpoint, i32::from(events))?;
    let reader = context.socket(PAIR)?;
    reader.connect(&endpoint)?;

    spawn(move || {
        while let Ok(frames) = reader.recv_multipart(0) {
            let Some(header) = frames.first().filter(|header| header.len() >= 2) else {
                continue;
            };
            let event = u16::from_ne_bytes([header[0], header[1]]);
            if event == SocketEvent::MONITOR_STOPPED.to_raw() {
                break;
            }
            if event == SocketEvent::ACCEPTED.to_raw() || event == SocketEvent::CONNECTED.to_raw() {
                count.fetch_add(1, Ordering::SeqCst);
            } else if event == SocketEvent::DISCONNECTED.to_raw() {
                decrement(&count);
            }
        }
    });
    Ok(())
}

fn decrement(count: &AtomicUsize) {
    let mut current = count.load(Ordering::SeqCst);
    while current > 0 {
        match count.compare_exchange(current, current - 1, Ordering::SeqCst, Ordering::SeqCst) {
            Ok(_) => break,
            Err(actual) => current = actual,
        }
    }
}
