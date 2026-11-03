# BestDeck (phone app)

The phone side of BestDeck: a grid of keys that run actions on your PC. It talks to the
[Windows host](https://github.com/vrgmrkjns/best-deck-windows) over the local network. No account, no cloud.

The whole app is Rust: [egui](https://github.com/emilk/egui) drawn through Android's `NativeActivity`, with no Java or Kotlin in the APK.

## Use

1. Start BestDeck on the PC.
2. Open the app on a phone on the same Wi-Fi. It finds the PC by itself (a UDP question on port 8191); tap it. If it does not show up, choose **Cím megadása kézzel** and type the address the editor's **Eszközök** tab shows.
3. Type the six-digit pairing code from the same tab, on the app's own keypad.

The app remembers the PC and reconnects on its own. Tap the PC's address on the deck screen to choose another one. Phones that had the earlier Flutter app keep their pairing.

## Layout

- `app/`: the app. `src/connection.rs` keeps the WebSocket link, `src/ui/` draws the two screens, `src/platform.rs` holds the few Android calls made through JNI (device name, wake lock, haptic tick, handing an update to the installer).
- `android/`: the manifest and launcher icons. The icons are rendered by `make-icons` in best-deck-windows.
- `xtask/`: builds, signs and publishes the APK.

## Build

Needs the Android SDK with an NDK, build-tools and a platform (found through `ANDROID_HOME`, or `%LOCALAPPDATA%\Android\Sdk`), `JAVA_HOME` for `apksigner`, and the Rust target:

```bash
rustup target add aarch64-linux-android x86_64-linux-android
```

```bash
cargo xtask apk
```

The APK lands in `target/apk/best-deck.apk`. `--emulator` adds the x86_64 library for the Android emulator, and `cargo xtask install` puts it on the device `adb` sees.

```bash
cargo test -p best-deck-mobile
```

```bash
cargo run -p best-deck-mobile
```

runs the app in a phone-sized window on Windows against a real host, which is the quick way to work on the screens. `BEST_DECK_PHONE_DATA` keeps that window's pairing apart.

## Releases and updates

Phones update themselves from the PC: the host serves the newest APK, and the app offers it when the host's build is newer than its own.

```bash
cargo xtask publish --bump
```

This raises `version-code` and the version in `app/Cargo.toml`, builds a signed release APK and copies it with its version and SHA-256 into `%APPDATA%\best-deck\updates`. The phone shows "Új verzió a gépen" on its next connection; one tap downloads it, checks the checksum, puts it in Downloads and opens the Android installer (Android 10 or newer; the first time, Android asks to allow installs from best-deck).

Android installs an update only when it is signed with the same key as the installed app. Release builds are signed with the key described in `android/key.properties` (not in git):

```
storeFile=C:/Users/<you>/.best-deck/release.jks
storePassword=...
keyAlias=best-deck
keyPassword=...
```

**Back up that file and the keystore it points to**: without them no phone can be updated, only reinstalled. Without `key.properties`, `apk` falls back to the debug key and `publish` refuses to run.

## Protocol

The app speaks the JSON protocol documented in the host's README. `app/src/protocol.rs` holds the message shapes and the version number sent in `hello`. The key icons in `app/src/icons.rs` mirror the host's `src/icons.rs`.
