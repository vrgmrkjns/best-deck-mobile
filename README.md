# best-deck (phone app)

The phone side of best-deck: a grid of keys that run actions on your PC. It talks to the
[Windows host](https://github.com/vrgmrkjns/best-deck-windows) over the local network. No account, no cloud.

## Use

1. Start best-deck on the PC and open the **Eszközök** tab of its editor.
2. In the app, type the address shown there, then the six-digit pairing code.

The app remembers the PC and reconnects on its own. Tap the PC's name on the deck screen to choose another one.

## Build

```bash
flutter build apk --release
```

The APK lands in `build/app/outputs/flutter-apk/app-release.apk`.

```bash
flutter test
```

## Releases and updates

Phones update themselves from the PC: the host serves the newest APK, and the app offers it when the host's build is newer than its own.

```bash
dart run tool/publish.dart --bump
```

This raises the build number, builds a signed release APK and copies it with its version and SHA-256 into `%APPDATA%\best-deck\updates`. The phone shows "Új verzió a gépen" on its next connection; one tap downloads it, checks the checksum and opens the Android installer.

Android installs an update only when it is signed with the same key as the installed app. Release builds are signed with the key described in `android/key.properties` (not in git). **Back up that file and the keystore it points to**: without them no phone can be updated, only reinstalled. Without `key.properties` a build falls back to the debug key, and `publish.dart` refuses to publish it.

## Protocol

The app speaks the JSON protocol documented in the host's README. `lib/protocol.dart` holds the message shapes and the version number sent in `hello`. The key icons in `lib/icons.dart` mirror the host's `web/icons.js`.
