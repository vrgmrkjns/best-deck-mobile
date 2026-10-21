import 'dart:async';
import 'dart:convert';

import 'package:flutter/foundation.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:web_socket_channel/web_socket_channel.dart';

import 'protocol.dart';

/// Where the app keeps the last host and the token each host issued.
abstract class Store {
  Future<String?> read(String key);
  Future<void> write(String key, String? value);
}

class PreferencesStore implements Store {
  @override
  Future<String?> read(String key) async => (await SharedPreferences.getInstance()).getString(key);

  @override
  Future<void> write(String key, String? value) async {
    final preferences = await SharedPreferences.getInstance();
    value == null ? await preferences.remove(key) : await preferences.setString(key, value);
  }
}

enum Phase {
  /// No host chosen.
  idle,

  /// First contact with a host is in progress.
  connecting,

  /// The host wants a pairing code.
  pairing,

  /// Paired; the deck is on screen. [DeckConnection.online] says if the link is up.
  ready,
}

typedef ChannelOpener = WebSocketChannel Function(Uri uri);

class DeckConnection extends ChangeNotifier {
  DeckConnection({required this.store, ChannelOpener? open, this.deviceName = 'Android telefon'})
    : _open = open ?? WebSocketChannel.connect;

  final Store store;
  final ChannelOpener _open;
  final String deviceName;

  Phase phase = Phase.idle;
  bool online = false;
  Uri? uri;
  String profile = '';
  List<DeckFolder> folders = const [];
  final Map<String, KeyState> states = {};

  /// Slider levels by key id, 0 to 100.
  final Map<String, int> levels = {};

  /// Last problem worth showing; cleared by the next successful step.
  String? error;

  WebSocketChannel? _channel;
  StreamSubscription<dynamic>? _subscription;
  Timer? _retry;
  int _attempt = 0;
  bool _disposed = false;

  String get hostLabel => uri == null ? '' : '${uri!.host}:${uri!.port}';
  String get _tokenKey => 'token:$hostLabel';

  /// Reconnects to the host used last time, if any.
  Future<void> restore() async {
    final saved = await store.read('host');
    if (saved != null) await connect(saved);
  }

  /// Returns false when [input] cannot be a host address.
  Future<bool> connect(String input) async {
    final target = clientUri(input);
    if (target == null) {
      error = 'Ez nem tűnik érvényes címnek.';
      notifyListeners();
      return false;
    }
    _close();
    uri = target;
    phase = Phase.connecting;
    error = null;
    _attempt = 0;
    notifyListeners();
    await store.write('host', hostLabel);
    await _dial();
    return true;
  }

  Future<void> _dial() async {
    final target = uri;
    if (target == null || _disposed) return;
    try {
      final channel = _open(target);
      _channel = channel;
      await channel.ready;
      if (_channel != channel) return; // superseded while connecting
      _subscription = channel.stream.listen(_receive, onError: (_) => _lost(channel), onDone: () => _lost(channel));
      online = true;
      _attempt = 0;
      _send({'type': 'hello', 'token': await store.read(_tokenKey), 'protocol': protocolVersion});
      notifyListeners();
    } catch (_) {
      _lost(_channel);
    }
  }

  void _lost(WebSocketChannel? channel) {
    if (_disposed || (channel != null && channel != _channel)) return;
    _subscription?.cancel();
    _subscription = null;
    _channel = null;
    online = false;
    if (uri == null) return;
    if (phase != Phase.ready) error = 'Nem érem el a gépet ezen a címen: $hostLabel';
    notifyListeners();
    // Keep trying: the PC may be waking up, or the phone just rejoined Wi-Fi.
    _attempt++;
    final seconds = _attempt > 3 ? 5 : _attempt;
    _retry?.cancel();
    _retry = Timer(Duration(seconds: seconds), _dial);
  }

  void _receive(dynamic data) {
    final Map<String, dynamic> message;
    try {
      message = jsonDecode(data as String) as Map<String, dynamic>;
    } catch (_) {
      return;
    }
    switch (message['type']) {
      case 'pair_required':
        phase = Phase.pairing;
        folders = const [];
        states.clear();
      case 'paired':
        store.write(_tokenKey, message['token'] as String);
        error = null;
      case 'deck':
        profile = message['profile'] as String? ?? '';
        folders = [for (final folder in message['folders'] as List) DeckFolder.fromJson(folder as Map<String, dynamic>)];
        for (final key in folders.expand((folder) => folder.keys)) {
          if (key.isSlider && key.value != null) levels[key.id] = key.value!;
        }
        phase = Phase.ready;
        error = null;
      case 'state':
        states[message['action'] as String] = KeyState(lit: message['lit'] as bool? ?? false, label: message['label'] as String?);
      case 'level':
        levels[message['key'] as String] = message['value'] as int;
      case 'error':
        error = _translate(message['message'] as String? ?? '');
      default:
        return;
    }
    notifyListeners();
  }

  static String _translate(String message) =>
      message == 'wrong pairing code' ? 'Nem ez a kód. Nézd meg újra a gépen.' : message;

  void _send(Map<String, dynamic> message) => _channel?.sink.add(jsonEncode(message));

  void pair(String code) {
    error = null;
    _send({'type': 'pair', 'code': code.trim(), 'name': deviceName});
    notifyListeners();
  }

  void press(String keyId) {
    if (!online) return;
    error = null;
    _send({'type': 'press', 'key': keyId});
  }

  void slide(String keyId, int value) {
    if (!online) return;
    levels[keyId] = value;
    _send({'type': 'slide', 'key': keyId, 'value': value});
  }

  /// Drops the current host so another one can be chosen. The pairing token is kept.
  Future<void> forget() async {
    _close();
    uri = null;
    phase = Phase.idle;
    error = null;
    folders = const [];
    states.clear();
    notifyListeners();
    await store.write('host', null);
  }

  void _close() {
    _retry?.cancel();
    _retry = null;
    _subscription?.cancel();
    _subscription = null;
    _channel?.sink.close();
    _channel = null;
    online = false;
  }

  @override
  void dispose() {
    _disposed = true;
    _close();
    super.dispose();
  }
}
