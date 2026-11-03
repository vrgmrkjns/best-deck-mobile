//! Builds, signs and publishes the Android app. Needs the Android SDK with an NDK, build-tools
//! and a platform, and the Rust targets `aarch64-linux-android` (and `x86_64-linux-android`
//! for the emulator).
//!
//!     cargo xtask apk [--emulator]          target/apk/best-deck.apk
//!     cargo xtask publish [--bump] [--to <folder>]
//!     cargo xtask install [--emulator]      onto the phone (or emulator) adb sees
//!
//! Release builds are signed with the key described in `android/key.properties` (not in git).
//! Android installs an update only when it is signed with the same key as the installed app.

use std::collections::BTreeMap;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, anyhow, bail, ensure};
use sha2::{Digest, Sha256};

/// Oldest Android the app runs on (7.0).
const MIN_SDK: u32 = 24;
const LIBRARY: &str = "libbest_deck.so";

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let flag = |name: &str| args.iter().any(|a| a == name);
    let option = |name: &str| args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).cloned();
    let abis: &[Abi] = if flag("--emulator") { &[Abi::ARM64, Abi::X86_64] } else { &[Abi::ARM64] };
    match args.first().map(String::as_str) {
        Some("apk") => {
            let apk = build_apk(abis, Signing::from_key_properties().or_else(|_| Signing::debug())?)?;
            println!("{}", apk.display());
        }
        Some("publish") => publish(flag("--bump"), option("--to"))?,
        Some("install") => {
            let apk = build_apk(abis, Signing::from_key_properties().or_else(|_| Signing::debug())?)?;
            let sdk = Sdk::find()?;
            run(Command::new(sdk.root.join("platform-tools").join("adb.exe")).args(["install", "-r"]).arg(&apk))?;
        }
        _ => bail!("usage: cargo xtask apk|publish|install [--emulator] [--bump] [--to <folder>]"),
    }
    Ok(())
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().expect("xtask sits in the workspace").to_path_buf()
}

#[derive(Clone, Copy)]
struct Abi {
    /// The folder name inside the APK.
    name: &'static str,
    rust: &'static str,
    clang: &'static str,
}

impl Abi {
    const ARM64: Abi = Abi { name: "arm64-v8a", rust: "aarch64-linux-android", clang: "aarch64-linux-android" };
    const X86_64: Abi = Abi { name: "x86_64", rust: "x86_64-linux-android", clang: "x86_64-linux-android" };
}

struct Sdk {
    root: PathBuf,
    ndk_bin: PathBuf,
    build_tools: PathBuf,
    android_jar: PathBuf,
    target_sdk: u32,
}

/// The highest-numbered folder inside `dir`, by its version numbers.
fn newest(dir: &Path, prefix: &str) -> Result<PathBuf> {
    let key = |path: &PathBuf| -> Vec<u32> {
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        name.trim_start_matches(prefix).split(|c: char| !c.is_ascii_digit()).filter_map(|n| n.parse().ok()).collect()
    };
    fs::read_dir(dir)
        .with_context(|| format!("{} is missing", dir.display()))?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.is_dir() && path.file_name().is_some_and(|n| n.to_string_lossy().starts_with(prefix)))
        .max_by_key(key)
        .ok_or_else(|| anyhow!("nothing in {}", dir.display()))
}

impl Sdk {
    fn find() -> Result<Sdk> {
        let root = std::env::var_os("ANDROID_HOME")
            .or_else(|| std::env::var_os("ANDROID_SDK_ROOT"))
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("LOCALAPPDATA").map(|dir| PathBuf::from(dir).join("Android").join("Sdk")))
            .filter(|dir| dir.is_dir())
            .ok_or_else(|| anyhow!("Android SDK not found; set ANDROID_HOME"))?;
        let ndk = newest(&root.join("ndk"), "")?;
        let platform = newest(&root.join("platforms"), "android-")?;
        let target_sdk = platform.file_name().unwrap().to_string_lossy().trim_start_matches("android-").parse()?;
        Ok(Sdk {
            ndk_bin: ndk.join("toolchains").join("llvm").join("prebuilt").join("windows-x86_64").join("bin"),
            build_tools: newest(&root.join("build-tools"), "")?,
            android_jar: platform.join("android.jar"),
            target_sdk,
            root,
        })
    }

    fn tool(&self, name: &str) -> PathBuf {
        let exe = self.build_tools.join(format!("{name}.exe"));
        if exe.exists() { exe } else { self.build_tools.join(format!("{name}.bat")) }
    }
}

struct Signing {
    keystore: PathBuf,
    alias: String,
    store_password: String,
    key_password: String,
    release: bool,
}

impl Signing {
    /// `storeFile`, `storePassword`, `keyAlias` and `keyPassword` from `android/key.properties`.
    fn from_key_properties() -> Result<Signing> {
        let path = root().join("android").join("key.properties");
        let text = fs::read_to_string(&path).with_context(|| format!("{} is missing", path.display()))?;
        let values: BTreeMap<&str, &str> = text.lines().filter_map(|line| line.split_once('=')).map(|(k, v)| (k.trim(), v.trim())).collect();
        let get = |key: &str| values.get(key).map(|v| v.to_string()).ok_or_else(|| anyhow!("{key} is missing from {}", path.display()));
        Ok(Signing {
            keystore: PathBuf::from(get("storeFile")?),
            alias: get("keyAlias")?,
            store_password: get("storePassword")?,
            key_password: get("keyPassword")?,
            release: true,
        })
    }

    /// The SDK's shared debug key, made on first use. Phones cannot update from it to a release build.
    fn debug() -> Result<Signing> {
        let home = std::env::var_os("USERPROFILE").map(PathBuf::from).ok_or_else(|| anyhow!("USERPROFILE is not set"))?;
        let keystore = home.join(".android").join("debug.keystore");
        if !keystore.exists() {
            fs::create_dir_all(keystore.parent().unwrap())?;
            run(Command::new(java_tool("keytool")?).args(["-genkeypair", "-keystore"]).arg(&keystore).args([
                "-storepass", "android", "-keypass", "android", "-alias", "androiddebugkey", "-keyalg", "RSA", "-keysize", "2048", "-validity", "10000",
                "-dname", "CN=Android Debug,O=Android,C=US",
            ]))?;
        }
        Ok(Signing { keystore, alias: "androiddebugkey".into(), store_password: "android".into(), key_password: "android".into(), release: false })
    }
}

fn java_tool(name: &str) -> Result<PathBuf> {
    let home = std::env::var_os("JAVA_HOME").map(PathBuf::from).ok_or_else(|| anyhow!("JAVA_HOME is not set"))?;
    Ok(home.join("bin").join(format!("{name}.exe")))
}

fn run(command: &mut Command) -> Result<()> {
    let status = command.status().with_context(|| format!("cannot start {:?}", command.get_program()))?;
    ensure!(status.success(), "{:?} failed with {status}", command.get_program());
    Ok(())
}

/// `version` and `version-code` from `app/Cargo.toml`.
fn app_version() -> Result<(String, u32)> {
    let manifest = fs::read_to_string(root().join("app").join("Cargo.toml"))?;
    let value = |key: &str| {
        manifest.lines().find_map(|line| Some(line.trim().strip_prefix(key)?.trim_start().strip_prefix('=')?.trim().trim_matches('"').to_owned()))
    };
    let name = value("version").ok_or_else(|| anyhow!("app/Cargo.toml has no version"))?;
    let code = value("version-code").ok_or_else(|| anyhow!("app/Cargo.toml has no version-code"))?.parse()?;
    Ok((name, code))
}

/// Writes a new build number and raises the last part of the version name with it,
/// so phones show which release they are offered.
fn set_version_code(code: u32) -> Result<()> {
    let path = root().join("app").join("Cargo.toml");
    let manifest = fs::read_to_string(&path)?;
    let (name, _) = app_version()?;
    let mut parts: Vec<u32> = name.split('.').map(|p| p.parse().unwrap_or(0)).collect();
    if let Some(last) = parts.last_mut() {
        *last += 1;
    }
    let name = parts.iter().map(u32::to_string).collect::<Vec<_>>().join(".");
    let updated: Vec<String> = manifest
        .lines()
        .map(|line| {
            if line.trim_start().starts_with("version-code") {
                format!("version-code = {code}")
            } else if line.starts_with("version =") {
                format!("version = \"{name}\"")
            } else {
                line.to_owned()
            }
        })
        .collect();
    fs::write(&path, updated.join("\n") + "\n")?;
    Ok(())
}

fn build_library(sdk: &Sdk, abi: Abi) -> Result<PathBuf> {
    let clang = sdk.ndk_bin.join(format!("{}{MIN_SDK}-clang.cmd", abi.clang));
    ensure!(clang.exists(), "{} is missing; install the NDK", clang.display());
    let env_target = abi.rust.replace('-', "_");
    let mut command = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()));
    command
        .current_dir(root())
        .args(["build", "--release", "--lib", "-p", "best-deck-mobile", "--target", abi.rust])
        .env(format!("CARGO_TARGET_{}_LINKER", env_target.to_uppercase()), &clang)
        .env(format!("CC_{env_target}"), &clang)
        .env(format!("AR_{env_target}"), sdk.ndk_bin.join("llvm-ar.exe"));
    run(&mut command)?;
    Ok(root().join("target").join(abi.rust).join("release").join(LIBRARY))
}

fn build_apk(abis: &[Abi], signing: Signing) -> Result<PathBuf> {
    let sdk = Sdk::find()?;
    let (version_name, version_code) = app_version()?;
    let out = root().join("target").join("apk");
    fs::create_dir_all(&out)?;

    let libraries: Vec<(Abi, PathBuf)> = abis.iter().map(|&abi| Ok((abi, build_library(&sdk, abi)?))).collect::<Result<_>>()?;

    // The manifest and launcher icons, compiled and linked against the platform.
    let resources = out.join("res.zip");
    run(Command::new(sdk.tool("aapt2")).arg("compile").arg("--dir").arg(root().join("android").join("res")).arg("-o").arg(&resources))?;
    let linked = out.join("linked.apk");
    run(Command::new(sdk.tool("aapt2"))
        .arg("link")
        .arg("-o")
        .arg(&linked)
        .arg("-I")
        .arg(&sdk.android_jar)
        .arg("--manifest")
        .arg(root().join("android").join("AndroidManifest.xml"))
        .args(["--min-sdk-version", &MIN_SDK.to_string(), "--target-sdk-version", &sdk.target_sdk.to_string()])
        .args(["--version-code", &version_code.to_string(), "--version-name", &version_name])
        .arg(&resources))?;

    // Native libraries go in uncompressed, so Android can map them straight from the APK.
    let unaligned = out.join("unaligned.apk");
    {
        let mut source = zip::ZipArchive::new(fs::File::open(&linked)?)?;
        let mut zip = zip::ZipWriter::new(fs::File::create(&unaligned)?);
        for i in 0..source.len() {
            zip.raw_copy_file(source.by_index_raw(i)?)?;
        }
        let stored = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        for (abi, library) in &libraries {
            zip.start_file(format!("lib/{}/{LIBRARY}", abi.name), stored)?;
            let mut bytes = Vec::new();
            fs::File::open(library)?.read_to_end(&mut bytes)?;
            zip.write_all(&bytes)?;
        }
        zip.finish()?;
    }

    // 16 KB pages, which Android 15 devices may use.
    let aligned = out.join("aligned.apk");
    run(Command::new(sdk.tool("zipalign")).args(["-f", "-P", "16", "4"]).arg(&unaligned).arg(&aligned))?;

    let apk = out.join("best-deck.apk");
    run(Command::new(sdk.tool("apksigner"))
        .arg("sign")
        .arg("--ks")
        .arg(&signing.keystore)
        .args(["--ks-key-alias", &signing.alias, "--ks-pass", "env:BEST_DECK_STORE_PASSWORD", "--key-pass", "env:BEST_DECK_KEY_PASSWORD"])
        .arg("--out")
        .arg(&apk)
        .arg(&aligned)
        .env("BEST_DECK_STORE_PASSWORD", &signing.store_password)
        .env("BEST_DECK_KEY_PASSWORD", &signing.key_password))?;
    for leftover in [&resources, &linked, &unaligned, &aligned] {
        let _ = fs::remove_file(leftover);
    }
    if !signing.release {
        eprintln!("note: signed with the debug key; phones with the release build cannot update to it");
    }
    Ok(apk)
}

/// Builds a signed release and copies it, with its version and checksum, to the host's updates folder.
fn publish(bump: bool, to: Option<String>) -> Result<()> {
    let target = to.map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(std::env::var_os("APPDATA").unwrap_or_default()).join("best-deck").join("updates")
    });
    let description = target.join("best-deck.json");
    let published = fs::read_to_string(&description)
        .ok()
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
        .and_then(|json| json["version_code"].as_u64())
        .unwrap_or(0) as u32;
    let (_, mut code) = app_version()?;
    if bump {
        code = code.max(published) + 1;
        set_version_code(code)?;
        println!("Build number raised to {code}, version to {}.", app_version()?.0);
    }
    ensure!(code > published, "build {code} is not newer than the published build {published}; raise version-code or pass --bump");
    let signing = Signing::from_key_properties().context("publishing needs the release key, so phones can update")?;

    let apk = build_apk(&[Abi::ARM64], signing)?;
    let (name, code) = app_version()?;
    let bytes = fs::read(&apk)?;
    let sha256: String = Sha256::digest(&bytes).iter().map(|b| format!("{b:02x}")).collect();
    fs::create_dir_all(&target)?;
    // The APK first and the description last, so the host never offers a half-copied file.
    fs::write(target.join("best-deck.apk"), &bytes)?;
    fs::write(&description, serde_json::to_string_pretty(&serde_json::json!({ "version_code": code, "version_name": name, "sha256": sha256 }))?)?;
    println!("Published {name} (build {code}) to {}.", target.display());
    Ok(())
}
