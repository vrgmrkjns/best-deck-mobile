//! Finds best-deck hosts on the local network: asks `best-deck?` by UDP broadcast on port 8191
//! and lists every PC that answers with its name and port.

use std::net::{Ipv4Addr, SocketAddr, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::Deserialize;

use crate::protocol::DEFAULT_PORT;

const QUESTION: &[u8] = b"best-deck?";
/// A PC that stops answering drops off the list after this long.
const FORGET_AFTER: Duration = Duration::from_secs(6);

#[derive(Clone, Debug, PartialEq)]
pub struct Found {
    pub name: String,
    /// `ip:port` to connect to.
    pub host: String,
    seen: Instant,
}

#[derive(Deserialize)]
struct Answer {
    best_deck: u32,
    name: String,
    port: u16,
}

/// Parses an answer that came from `from`.
fn parse(bytes: &[u8], from: SocketAddr) -> Option<Found> {
    let answer: Answer = serde_json::from_slice(bytes).ok()?;
    (answer.best_deck == 1).then(|| Found { name: answer.name, host: format!("{}:{}", from.ip(), answer.port), seen: Instant::now() })
}

#[derive(Default)]
pub struct Finder {
    found: Arc<Mutex<Vec<Found>>>,
    running: Arc<AtomicBool>,
}

impl Finder {
    /// The PCs heard from recently, by name.
    pub fn found(&self) -> Vec<Found> {
        let mut found = self.found.lock().unwrap().clone();
        found.retain(|f| f.seen.elapsed() < FORGET_AFTER);
        found.sort_by(|a, b| a.name.cmp(&b.name).then(a.host.cmp(&b.host)));
        found
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Starts asking around until [`Finder::stop`]. `waker` runs when the list changes.
    pub fn start(&self, waker: impl Fn() + Send + 'static) {
        if self.running.swap(true, Ordering::SeqCst) {
            return;
        }
        let (found, running) = (self.found.clone(), self.running.clone());
        std::thread::Builder::new()
            .name("discovery".into())
            .spawn(move || {
                let Ok(socket) = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)) else {
                    running.store(false, Ordering::SeqCst);
                    return;
                };
                let _ = socket.set_broadcast(true);
                let _ = socket.set_read_timeout(Some(Duration::from_millis(250)));
                let mut asked = Instant::now() - Duration::from_secs(10);
                let mut buffer = [0u8; 512];
                while running.load(Ordering::SeqCst) {
                    if asked.elapsed() >= Duration::from_millis(1500) {
                        asked = Instant::now();
                        let _ = socket.send_to(QUESTION, (Ipv4Addr::BROADCAST, DEFAULT_PORT));
                        // Drop PCs that went quiet.
                        let mut list = found.lock().unwrap();
                        let before = list.len();
                        list.retain(|f| f.seen.elapsed() < FORGET_AFTER);
                        if list.len() != before {
                            waker();
                        }
                    }
                    if let Ok((size, from)) = socket.recv_from(&mut buffer) {
                        if let Some(pc) = parse(&buffer[..size], from) {
                            let mut list = found.lock().unwrap();
                            let new = !list.iter().any(|f| f.host == pc.host);
                            list.retain(|f| f.host != pc.host);
                            list.push(pc);
                            drop(list);
                            if new {
                                waker();
                            }
                        }
                    }
                }
            })
            .ok();
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }
}

impl Drop for Finder {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_a_hosts_answer() {
        let from: SocketAddr = "192.168.0.106:8191".parse().unwrap();
        let pc = parse(br#"{"best_deck":1,"name":"ASZTALI","port":8191}"#, from).unwrap();
        assert_eq!(pc.name, "ASZTALI");
        assert_eq!(pc.host, "192.168.0.106:8191");
        assert!(parse(br#"{"best_deck":2,"name":"x","port":1}"#, from).is_none());
        assert!(parse(b"hello", from).is_none());
    }
}
