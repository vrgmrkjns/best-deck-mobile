//! Where the app keeps the last host and the token each host issued: `best-deck.json` in the
//! app's private files. On first start it takes them over from the earlier Flutter app.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Debug, PartialEq)]
pub struct Saved {
    /// `host:port` of the PC used last.
    #[serde(default)]
    pub host: Option<String>,
    /// Pairing tokens by `host:port`.
    #[serde(default)]
    pub tokens: BTreeMap<String, String>,
}

pub struct Store {
    path: PathBuf,
    pub saved: Saved,
}

impl Store {
    /// Loads from `dir`, or migrates the Flutter app's preferences found next to it.
    pub fn open(dir: &Path) -> Store {
        let path = dir.join("best-deck.json");
        let saved = match std::fs::read_to_string(&path) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_default(),
            Err(_) => {
                let migrated = dir
                    .parent()
                    .map(|app| app.join("shared_prefs").join("FlutterSharedPreferences.xml"))
                    .and_then(|file| std::fs::read_to_string(file).ok())
                    .map(|xml| from_flutter(&xml))
                    .unwrap_or_default();
                migrated
            }
        };
        let store = Store { path, saved };
        store.save();
        store
    }

    pub fn token(&self, host: &str) -> Option<&str> {
        self.saved.tokens.get(host).map(String::as_str)
    }

    pub fn set_host(&mut self, host: Option<String>) {
        self.saved.host = host;
        self.save();
    }

    pub fn set_token(&mut self, host: &str, token: String) {
        self.saved.tokens.insert(host.to_owned(), token);
        self.save();
    }

    fn save(&self) {
        if let Some(dir) = self.path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(text) = serde_json::to_string_pretty(&self.saved) {
            // Write then rename, so a crash never leaves half a file.
            let temp = self.path.with_extension("json.new");
            if std::fs::write(&temp, text).is_ok() {
                let _ = std::fs::rename(&temp, &self.path);
            }
        }
    }
}

/// Reads `flutter.host` and `flutter.token:<host>` from the Flutter app's SharedPreferences XML.
fn from_flutter(xml: &str) -> Saved {
    let mut saved = Saved::default();
    for line in xml.split("<string ").skip(1) {
        let Some(name) = line.split("name=\"").nth(1).and_then(|rest| rest.split('"').next()) else { continue };
        let Some(value) = line.split_once('>').and_then(|(_, rest)| rest.split("</string>").next()) else { continue };
        let value = unescape(value);
        if name == "flutter.host" {
            saved.host = Some(value);
        } else if let Some(host) = name.strip_prefix("flutter.token:") {
            saved.tokens.insert(unescape(host), value);
        }
    }
    saved
}

fn unescape(text: &str) -> String {
    text.replace("&quot;", "\"").replace("&apos;", "'").replace("&lt;", "<").replace("&gt;", ">").replace("&amp;", "&")
}

#[cfg(test)]
mod tests {
    use super::*;

    const FLUTTER: &str = r#"<?xml version='1.0' encoding='utf-8' standalone='yes' ?>
<map>
    <string name="flutter.host">192.168.0.106:8191</string>
    <string name="flutter.token:192.168.0.106:8191">abc123</string>
    <string name="flutter.other">x</string>
</map>"#;

    #[test]
    fn takes_over_the_flutter_apps_pairing() {
        let saved = from_flutter(FLUTTER);
        assert_eq!(saved.host.as_deref(), Some("192.168.0.106:8191"));
        assert_eq!(saved.tokens.get("192.168.0.106:8191").map(String::as_str), Some("abc123"));
        assert_eq!(saved.tokens.len(), 1);
    }

    #[test]
    fn migrates_once_then_keeps_its_own_file() {
        let root = std::env::temp_dir().join(format!("best-deck-store-{}", std::process::id()));
        let files = root.join("files");
        std::fs::create_dir_all(root.join("shared_prefs")).unwrap();
        std::fs::write(root.join("shared_prefs").join("FlutterSharedPreferences.xml"), FLUTTER).unwrap();

        let mut store = Store::open(&files);
        assert_eq!(store.token("192.168.0.106:8191"), Some("abc123"));
        store.set_host(None);
        store.set_token("pc:8191", "t".into());

        let again = Store::open(&files);
        assert_eq!(again.saved.host, None, "the Flutter file is not read twice");
        assert_eq!(again.token("pc:8191"), Some("t"));
        std::fs::remove_dir_all(root).unwrap();
    }
}
