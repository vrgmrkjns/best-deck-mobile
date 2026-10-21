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

The APK lands in `build/app/outputs/flutter-apk/app-release.apk`. Release builds are signed with the debug key for now, which is fine for installing on your own phone but not for a store.

```bash
flutter test
```

## Protocol

The app speaks the JSON protocol documented in the host's README. `lib/protocol.dart` holds the message shapes and the version number sent in `hello`. The key icons in `lib/icons.dart` mirror the host's `web/icons.js`.
