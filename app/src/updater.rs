//! Asks the host whether it has a newer build of this app, and installs it on request.

use std::io::Read;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::platform::Platform;

/// This build's Android version code, from `[package.metadata.android]` in `app/Cargo.toml`.
pub const VERSION_CODE: u32 = parse_code(env!("BEST_DECK_VERSION_CODE"));

const fn parse_code(text: &str) -> u32 {
    let bytes = text.as_bytes();
    let mut value = 0u32;
    let mut i = 0;
    while i < bytes.len() {
        value = value * 10 + (bytes[i] - b'0') as u32;
        i += 1;
    }
    value
}

/// A phone app build the host offers, from its `/app/version`.
#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct Release {
    pub version_code: u32,
    pub version_name: String,
    pub sha256: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Update {
    /// Nothing newer, or not asked yet.
    None,
    Available(Release),
    /// Downloading, 0 to 1.
    Downloading(Release, f32),
    /// The system installer is showing.
    Installing(Release),
    Failed(Release, String),
}

#[derive(Clone)]
pub struct Updater {
    state: Arc<Mutex<Update>>,
    host: Arc<Mutex<Option<String>>>,
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder().timeout_connect(Some(Duration::from_secs(5))).build().into()
}

impl Updater {
    pub fn new() -> Self {
        Updater { state: Arc::new(Mutex::new(Update::None)), host: Arc::default() }
    }

    pub fn state(&self) -> Update {
        self.state.lock().unwrap().clone()
    }

    /// Looks for a newer build on `host` (`ip:port`), in the background.
    pub fn check(&self, host: &str, waker: impl Fn() + Send + 'static) {
        let updater = self.clone();
        let host = host.to_owned();
        std::thread::spawn(move || {
            let release = agent()
                .get(format!("http://{host}/app/version"))
                .call()
                .ok()
                .and_then(|mut response| response.body_mut().read_to_string().ok())
                .and_then(|text| serde_json::from_str::<Release>(&text).ok());
            let mut state = updater.state.lock().unwrap();
            if matches!(*state, Update::Downloading(..)) {
                return;
            }
            *updater.host.lock().unwrap() = Some(host);
            // An older host without updates, or a flaky link: nothing to offer this time.
            *state = match release {
                Some(release) if release.version_code > VERSION_CODE => Update::Available(release),
                _ => Update::None,
            };
            drop(state);
            waker();
        });
    }

    /// Downloads the offered build, checks it and hands it to the system installer.
    pub fn start(&self, platform: Platform, waker: impl Fn() + Send + 'static) {
        let release = match self.state() {
            Update::Available(release) | Update::Failed(release, _) | Update::Installing(release) => release,
            _ => return,
        };
        let Some(host) = self.host.lock().unwrap().clone() else { return };
        *self.state.lock().unwrap() = Update::Downloading(release.clone(), 0.0);
        waker();
        let updater = self.clone();
        std::thread::spawn(move || {
            let result = download(&host, &release, |progress| {
                *updater.state.lock().unwrap() = Update::Downloading(release.clone(), progress);
                waker();
            })
            .and_then(|apk| platform.install(&apk, release.version_code));
            *updater.state.lock().unwrap() = match result {
                Ok(()) => Update::Installing(release),
                Err(error) => Update::Failed(release, error),
            };
            waker();
        });
    }
}

fn download(host: &str, release: &Release, progress: impl Fn(f32)) -> Result<Vec<u8>, String> {
    let failed = |_| "A frissítés nem sikerült: a gép nem adta át a fájlt.".to_owned();
    let mut response = agent().get(format!("http://{host}/app/best-deck.apk")).call().map_err(failed)?;
    let total = response.body().content_length().unwrap_or(0) as usize;
    let mut reader = response.body_mut().as_reader();
    let mut apk = Vec::with_capacity(total);
    let mut buffer = vec![0u8; 64 * 1024];
    let mut shown = 0.0;
    loop {
        let read = reader.read(&mut buffer).map_err(|_| "A letöltés megszakadt. Próbáld újra.".to_owned())?;
        if read == 0 {
            break;
        }
        apk.extend_from_slice(&buffer[..read]);
        if total > 0 {
            let now = apk.len() as f32 / total as f32;
            if now - shown >= 0.01 {
                shown = now;
                progress(now);
            }
        }
    }
    if !checksum_matches(&apk, &release.sha256) {
        return Err("A letöltött fájl sérült. Próbáld újra.".into());
    }
    Ok(apk)
}

fn checksum_matches(bytes: &[u8], expected: &str) -> bool {
    let digest = Sha256::digest(bytes);
    let hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();
    hex.eq_ignore_ascii_case(expected.trim())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checks_the_download() {
        let sha = "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";
        assert!(checksum_matches(b"hello", sha));
        assert!(checksum_matches(b"hello", &sha.to_uppercase()));
        assert!(!checksum_matches(b"hellO", sha));
        assert_eq!(parse_code("42"), 42);
    }
}
