//! The link to one host: a background thread keeps the WebSocket open, reconnects on its own
//! and folds what the host says into a [`Model`] the screens draw from.

use std::collections::HashMap;
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use serde_json::Value;
use tungstenite::{Message, WebSocket};

use crate::protocol::{self, DeckFolder, HostMsg, KeyState};
use crate::store::Store;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// No host chosen.
    Idle,
    /// First contact with a host is in progress.
    Connecting,
    /// The host wants a pairing code.
    Pairing,
    /// Paired; the deck is on screen. [`Model::online`] says if the link is up.
    Ready,
}

#[derive(Debug)]
pub struct Model {
    pub phase: Phase,
    pub online: bool,
    /// `host:port` of the chosen PC.
    pub host: Option<String>,
    pub profile: String,
    pub folders: Vec<DeckFolder>,
    pub states: HashMap<String, KeyState>,
    /// Slider levels by key id, 0 to 100.
    pub levels: HashMap<String, u8>,
    /// Last problem worth showing; cleared by the next successful step.
    pub error: Option<String>,
    /// Counts the times the deck arrived over a fresh link, so updates are looked for once per link.
    pub links: u64,
}

impl Model {
    fn new() -> Self {
        Model {
            phase: Phase::Idle,
            online: false,
            host: None,
            profile: String::new(),
            folders: Vec::new(),
            states: HashMap::new(),
            levels: HashMap::new(),
            error: None,
            links: 0,
        }
    }
}

type Waker = Arc<dyn Fn() + Send + Sync>;

pub struct Connection {
    model: Arc<Mutex<Model>>,
    store: Arc<Mutex<Store>>,
    outgoing: Option<Sender<Value>>,
    /// Bumped whenever the host changes; an older worker thread sees it and stops.
    generation: Arc<AtomicU64>,
    waker: Waker,
    device_name: String,
}

impl Connection {
    pub fn new(store: Store, device_name: String, waker: impl Fn() + Send + Sync + 'static) -> Self {
        Connection {
            model: Arc::new(Mutex::new(Model::new())),
            store: Arc::new(Mutex::new(store)),
            outgoing: None,
            generation: Arc::new(AtomicU64::new(0)),
            waker: Arc::new(waker),
            device_name,
        }
    }

    pub fn model(&self) -> MutexGuard<'_, Model> {
        self.model.lock().unwrap()
    }

    /// Reconnects to the host used last time, if any.
    pub fn restore(&mut self) {
        let saved = self.store.lock().unwrap().saved.host.clone();
        if let Some(host) = saved {
            self.connect(&host);
        }
    }

    /// Returns false when `input` cannot be a host address.
    pub fn connect(&mut self, input: &str) -> bool {
        let Some(host) = protocol::host_label(input) else {
            self.model().error = Some("Ez nem tűnik érvényes címnek.".into());
            return false;
        };
        let generation = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
        {
            let mut model = self.model();
            *model = Model { links: model.links, ..Model::new() };
            model.host = Some(host.clone());
            model.phase = Phase::Connecting;
        }
        self.store.lock().unwrap().set_host(Some(host.clone()));
        let (sender, receiver) = channel();
        self.outgoing = Some(sender);
        let worker = Worker {
            host,
            generation,
            current: self.generation.clone(),
            model: self.model.clone(),
            store: self.store.clone(),
            outgoing: receiver,
            waker: self.waker.clone(),
        };
        std::thread::Builder::new().name("host link".into()).spawn(move || worker.run()).expect("cannot start the link thread");
        true
    }

    fn send(&self, message: Value) {
        if let Some(outgoing) = &self.outgoing {
            let _ = outgoing.send(message);
        }
    }

    pub fn pair(&self, code: &str) {
        self.model().error = None;
        self.send(protocol::pair(code, &self.device_name));
    }

    pub fn press(&self, key: &str) {
        let mut model = self.model();
        if !model.online {
            return;
        }
        model.error = None;
        drop(model);
        self.send(protocol::press(key));
    }

    pub fn slide(&self, key: &str, value: u8) {
        let mut model = self.model();
        if !model.online {
            return;
        }
        model.levels.insert(key.to_owned(), value);
        drop(model);
        self.send(protocol::slide(key, value));
    }

    /// Drops the current host so another one can be chosen. The pairing token is kept.
    pub fn forget(&mut self) {
        self.generation.fetch_add(1, Ordering::SeqCst);
        self.outgoing = None;
        {
            let mut model = self.model();
            *model = Model { links: model.links, ..Model::new() };
        }
        self.store.lock().unwrap().set_host(None);
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        self.generation.fetch_add(1, Ordering::SeqCst);
    }
}

struct Worker {
    host: String,
    generation: u64,
    current: Arc<AtomicU64>,
    model: Arc<Mutex<Model>>,
    store: Arc<Mutex<Store>>,
    outgoing: Receiver<Value>,
    waker: Waker,
}

impl Worker {
    fn alive(&self) -> bool {
        self.current.load(Ordering::SeqCst) == self.generation
    }

    fn run(self) {
        let mut attempt = 0u32;
        while self.alive() {
            if let Ok(socket) = open(&self.host) {
                attempt = 0;
                self.serve(socket);
            }
            if !self.alive() {
                return;
            }
            {
                let mut model = self.model.lock().unwrap();
                model.online = false;
                if model.phase != Phase::Ready {
                    model.error = Some(format!("Nem érem el a gépet ezen a címen: {}", self.host));
                }
            }
            (self.waker)();
            // Keep trying: the PC may be waking up, or the phone just rejoined Wi-Fi.
            attempt += 1;
            let wait = Duration::from_secs(if attempt > 3 { 5 } else { u64::from(attempt) });
            let until = Instant::now() + wait;
            while Instant::now() < until && self.alive() {
                std::thread::sleep(Duration::from_millis(100));
            }
        }
    }

    /// Talks to the host until the link drops or the host is changed.
    fn serve(&self, mut socket: WebSocket<TcpStream>) {
        // Anything queued while offline is stale.
        while self.outgoing.try_recv().is_ok() {}
        let token = self.store.lock().unwrap().token(&self.host).map(str::to_owned);
        if send(&mut socket, &protocol::hello(token.as_deref())).is_err() {
            return;
        }
        self.model.lock().unwrap().online = true;
        (self.waker)();
        loop {
            if !self.alive() {
                let _ = socket.close(None);
                let _ = socket.flush();
                return;
            }
            while let Ok(message) = self.outgoing.try_recv() {
                if send(&mut socket, &message).is_err() {
                    return;
                }
            }
            match socket.read() {
                Ok(Message::Text(text)) => self.receive(&text),
                Ok(Message::Close(_)) => return,
                Ok(_) => {}
                Err(tungstenite::Error::Io(error)) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    let _ = socket.flush();
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(_) => return,
            }
        }
    }

    fn receive(&self, text: &str) {
        let Ok(message) = serde_json::from_str::<HostMsg>(text) else { return };
        let mut model = self.model.lock().unwrap();
        if !self.alive() {
            return;
        }
        match message {
            HostMsg::PairRequired => {
                model.phase = Phase::Pairing;
                model.folders.clear();
                model.states.clear();
            }
            HostMsg::Paired { token } => {
                self.store.lock().unwrap().set_token(&self.host, token);
                model.error = None;
            }
            HostMsg::Deck { profile, folders } => {
                for key in folders.iter().flat_map(|f| &f.keys) {
                    if let (true, Some(value)) = (key.is_slider(), key.value) {
                        model.levels.insert(key.id.clone(), value);
                    }
                }
                if model.phase != Phase::Ready {
                    model.links += 1;
                }
                model.profile = profile;
                model.folders = folders;
                model.phase = Phase::Ready;
                model.error = None;
            }
            HostMsg::State { action, lit, label } => {
                model.states.insert(action, KeyState { lit, label });
            }
            HostMsg::Level { key, value } => {
                model.levels.insert(key, value);
            }
            HostMsg::Error { message } => model.error = Some(translate(&message)),
            HostMsg::Unknown => return,
        }
        drop(model);
        (self.waker)();
    }
}

fn translate(message: &str) -> String {
    match message {
        "wrong pairing code" => "Nem ez a kód. Nézd meg újra a gépen.".into(),
        _ => message.to_owned(),
    }
}

fn open(host: &str) -> Result<WebSocket<TcpStream>, Box<dyn std::error::Error>> {
    let address = host.to_socket_addrs()?.next().ok_or("no address")?;
    let stream = TcpStream::connect_timeout(&address, Duration::from_secs(3))?;
    stream.set_nodelay(true)?;
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    let (socket, _) = tungstenite::client::client(format!("ws://{host}/client"), stream)?;
    socket.get_ref().set_read_timeout(None)?;
    // Non-blocking, so one thread can both listen and send.
    socket.get_ref().set_nonblocking(true)?;
    Ok(socket)
}

fn send(socket: &mut WebSocket<TcpStream>, message: &Value) -> Result<(), tungstenite::Error> {
    match socket.send(Message::text(message.to_string())) {
        // Queued; the next flush sends the rest.
        Err(tungstenite::Error::Io(error)) if error.kind() == std::io::ErrorKind::WouldBlock => Ok(()),
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    /// A stand-in host on a loopback port that runs `script` for the first connection.
    fn fake_host(script: impl FnOnce(&mut WebSocket<TcpStream>) + Send + 'static) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let host = listener.local_addr().unwrap().to_string();
        std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut socket = tungstenite::accept(stream).unwrap();
            script(&mut socket);
            std::thread::sleep(Duration::from_millis(300));
        });
        host
    }

    fn read_json(socket: &mut WebSocket<TcpStream>) -> Value {
        loop {
            if let Message::Text(text) = socket.read().unwrap() {
                return serde_json::from_str(&text).unwrap();
            }
        }
    }

    fn wait_for(connection: &Connection, check: impl Fn(&Model) -> bool) {
        let until = Instant::now() + Duration::from_secs(5);
        while Instant::now() < until {
            if check(&connection.model()) {
                return;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        panic!("timed out: {:?}", connection.model());
    }

    fn temp_store(name: &str) -> Store {
        Store::open(&std::env::temp_dir().join(format!("best-deck-link-{name}-{}", std::process::id())).join("files"))
    }

    #[test]
    fn pairs_then_shows_the_deck_and_presses() {
        let host = fake_host(|socket| {
            assert_eq!(read_json(socket)["type"], "hello");
            socket.send(Message::text(r#"{"type":"pair_required"}"#)).unwrap();
            let pair = read_json(socket);
            assert_eq!(pair, serde_json::json!({ "type": "pair", "code": "123456", "name": "teszt" }));
            socket.send(Message::text(r#"{"type":"paired","token":"tok"}"#)).unwrap();
            socket
                .send(Message::text(
                    r#"{"type":"deck","profile":"P","folders":[{"name":"F","columns":4,"rows":6,"keys":[{"id":"s","row":0,"col":0,"kind":"slider","span":2,"value":30}]}]}"#,
                ))
                .unwrap();
            socket.send(Message::text(r#"{"type":"state","action":"key.m","lit":true,"label":"Némítva"}"#)).unwrap();
            assert_eq!(read_json(socket), serde_json::json!({ "type": "press", "key": "m" }));
            assert_eq!(read_json(socket), serde_json::json!({ "type": "slide", "key": "s", "value": 70 }));
        });
        let mut connection = Connection::new(temp_store("pair"), "teszt".into(), || {});
        assert!(connection.connect(&host));
        wait_for(&connection, |m| m.phase == Phase::Pairing);
        connection.pair(" 123456 ");
        wait_for(&connection, |m| m.phase == Phase::Ready && m.states.contains_key("key.m"));
        {
            let model = connection.model();
            assert_eq!(model.profile, "P");
            assert_eq!(model.levels.get("s"), Some(&30));
            assert_eq!(model.links, 1);
            assert_eq!(model.states["key.m"].label.as_deref(), Some("Némítva"));
        }
        assert_eq!(connection.store.lock().unwrap().token(&host), Some("tok"));
        connection.press("m");
        connection.slide("s", 70);
        wait_for(&connection, |m| m.levels.get("s") == Some(&70));
        std::thread::sleep(Duration::from_millis(200));
    }

    #[test]
    fn says_so_when_nobody_answers() {
        let free = TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().to_string();
        let mut connection = Connection::new(temp_store("none"), "teszt".into(), || {});
        assert!(!connection.connect("not an address!"));
        assert!(connection.model().error.is_some());
        assert!(connection.connect(&free));
        wait_for(&connection, |m| m.error.as_deref().is_some_and(|e| e.contains(&free)));
        assert_eq!(connection.model().phase, Phase::Connecting);
        connection.forget();
        assert_eq!(connection.model().phase, Phase::Idle);
        assert_eq!(connection.store.lock().unwrap().saved.host, None);
    }
}
