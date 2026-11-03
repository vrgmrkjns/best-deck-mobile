//! Messages of the host's `/client` WebSocket endpoint, as documented in the best-deck-windows README.

use serde::Deserialize;
use serde_json::{Value, json};

/// Sent in `hello` so a host can refuse a client it no longer understands.
pub const PROTOCOL_VERSION: u32 = 1;

/// The host's port when the user types none.
pub const DEFAULT_PORT: u16 = 8191;

#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct DeckKey {
    pub id: String,
    pub row: u8,
    pub col: u8,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub icon: String,
    /// Colour group: voice, capture, media, game or sys.
    #[serde(default)]
    pub group: String,
    /// False while a plugin this key needs is not connected on the host.
    #[serde(default = "yes")]
    pub available: bool,
    /// `button` or `slider`.
    #[serde(default = "button")]
    pub kind: String,
    /// How many grid cells the key covers to the right.
    #[serde(default = "one")]
    pub span: u8,
    /// A slider's level (0 to 100) when the host could read it.
    #[serde(default)]
    pub value: Option<u8>,
    /// The `plugin.action` whose state lights this key.
    #[serde(default)]
    pub lit_when: Option<String>,
    #[serde(default)]
    pub lit_label: Option<String>,
    /// Icon while lit, such as a crossed-out microphone; the normal icon otherwise.
    #[serde(default)]
    pub lit_icon: Option<String>,
    /// The folder, by position, to show when the key is pressed.
    #[serde(default)]
    pub open_folder: Option<usize>,
    /// Go back to the folder shown before.
    #[serde(default)]
    pub back: bool,
}

fn yes() -> bool {
    true
}

fn button() -> String {
    "button".into()
}

fn one() -> u8 {
    1
}

impl DeckKey {
    pub fn is_slider(&self) -> bool {
        self.kind == "slider"
    }

    pub fn span(&self) -> u8 {
        self.span.max(1)
    }
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct DeckFolder {
    #[serde(default)]
    pub name: String,
    pub columns: u8,
    pub rows: u8,
    pub keys: Vec<DeckKey>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct KeyState {
    pub lit: bool,
    pub label: Option<String>,
}

/// What the host sends.
#[derive(Deserialize, Debug, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HostMsg {
    PairRequired,
    Paired {
        token: String,
    },
    Deck {
        #[serde(default)]
        profile: String,
        folders: Vec<DeckFolder>,
    },
    State {
        action: String,
        #[serde(default)]
        lit: bool,
        #[serde(default)]
        label: Option<String>,
    },
    Level {
        key: String,
        value: u8,
    },
    Error {
        #[serde(default)]
        message: String,
    },
    /// A message from a newer host; ignored.
    #[serde(other)]
    Unknown,
}

pub fn hello(token: Option<&str>) -> Value {
    json!({ "type": "hello", "token": token, "protocol": PROTOCOL_VERSION })
}

pub fn pair(code: &str, name: &str) -> Value {
    json!({ "type": "pair", "code": code.trim(), "name": name })
}

pub fn press(key: &str) -> Value {
    json!({ "type": "press", "key": key })
}

pub fn slide(key: &str, value: u8) -> Value {
    json!({ "type": "slide", "key": key, "value": value })
}

/// Turns what the user typed (`192.168.0.110`, `192.168.0.110:8191`, `http://pc:8191/`)
/// into `host:port`, or `None` if it cannot be an address.
pub fn host_label(input: &str) -> Option<String> {
    let mut text = input.trim();
    if let Some((scheme, rest)) = text.split_once("://") {
        if scheme.chars().all(|c| c.is_ascii_alphabetic()) {
            text = rest;
        }
    }
    let text = text.split('/').next().unwrap_or("");
    let (host, port) = match text.rsplit_once(':') {
        Some((host, port)) => (host, port.parse::<u16>().ok().filter(|p| *p > 0)?),
        None => (text, DEFAULT_PORT),
    };
    let valid = !host.is_empty() && host.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-');
    valid.then(|| format!("{host}:{port}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_what_people_type_as_an_address() {
        assert_eq!(host_label("192.168.0.110").as_deref(), Some("192.168.0.110:8191"));
        assert_eq!(host_label(" 192.168.0.110:9000 ").as_deref(), Some("192.168.0.110:9000"));
        assert_eq!(host_label("http://pc:8191/").as_deref(), Some("pc:8191"));
        assert_eq!(host_label(""), None);
        assert_eq!(host_label("192.168.0.110:abc"), None);
        assert_eq!(host_label("192.168.0.110:0"), None);
        assert_eq!(host_label("a b"), None);
    }

    #[test]
    fn reads_the_hosts_messages() {
        let deck: HostMsg = serde_json::from_str(
            r#"{"type":"deck","profile":"Gaming","folders":[{"name":"F","columns":4,"rows":6,"keys":[
                {"id":"v","row":4,"col":0,"label":"Hangerő","icon":"volume","group":"media","kind":"slider","span":2,"value":40,"available":true},
                {"id":"m","row":0,"col":0,"label":"Némítás","icon":"mic","group":"voice","kind":"button","span":1,"available":false,"lit_when":"key.m","lit_icon":"mic-off"}]}]}"#,
        )
        .unwrap();
        let HostMsg::Deck { profile, folders } = deck else { panic!("not a deck") };
        assert_eq!(profile, "Gaming");
        assert!(folders[0].keys[0].is_slider());
        assert_eq!(folders[0].keys[0].span(), 2);
        assert_eq!(folders[0].keys[0].value, Some(40));
        assert!(!folders[0].keys[1].available);
        assert_eq!(folders[0].keys[1].lit_icon.as_deref(), Some("mic-off"));

        let state: HostMsg = serde_json::from_str(r#"{"type":"state","action":"key.m","lit":true}"#).unwrap();
        assert_eq!(state, HostMsg::State { action: "key.m".into(), lit: true, label: None });
        let folder: DeckKey = serde_json::from_str(r#"{"id":"f","row":0,"col":0,"open_folder":2}"#).unwrap();
        assert_eq!((folder.open_folder, folder.back), (Some(2), false));
        let back: DeckKey = serde_json::from_str(r#"{"id":"b","row":0,"col":0,"back":true}"#).unwrap();
        assert_eq!((back.open_folder, back.back), (None, true));
        let unknown: HostMsg = serde_json::from_str(r#"{"type":"something_new","x":1}"#).unwrap();
        assert_eq!(unknown, HostMsg::Unknown);
    }
}
