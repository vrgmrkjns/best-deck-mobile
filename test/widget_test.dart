import 'dart:async';
import 'dart:convert';

import 'package:best_deck/connection.dart';
import 'package:best_deck/main.dart';
import 'package:best_deck/protocol.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:web_socket_channel/web_socket_channel.dart';

class MemoryStore implements Store {
  final values = <String, String>{};

  @override
  Future<String?> read(String key) async => values[key];

  @override
  Future<void> write(String key, String? value) async => value == null ? values.remove(key) : values[key] = value;
}

/// Stands in for the host: records what the app sends and lets a test answer.
class FakeChannel implements WebSocketChannel {
  final _incoming = StreamController<dynamic>();
  final sent = <Map<String, dynamic>>[];
  late final _sink = _FakeSink(sent);

  void host(Map<String, dynamic> message) => _incoming.add(jsonEncode(message));

  @override
  Future<void> get ready => Future.value();

  @override
  Stream<dynamic> get stream => _incoming.stream;

  @override
  WebSocketSink get sink => _sink;

  @override
  dynamic noSuchMethod(Invocation invocation) => super.noSuchMethod(invocation);
}

class _FakeSink implements WebSocketSink {
  _FakeSink(this.sent);

  final List<Map<String, dynamic>> sent;

  @override
  void add(dynamic data) => sent.add(jsonDecode(data as String) as Map<String, dynamic>);

  @override
  Future<dynamic> close([int? closeCode, String? closeReason]) async {}

  @override
  dynamic noSuchMethod(Invocation invocation) => super.noSuchMethod(invocation);
}

const deck = {
  'type': 'deck',
  'profile': 'Gaming',
  'folders': [
    {
      'name': 'Főoldal',
      'columns': 4,
      'rows': 6,
      'keys': [
        {'id': 'mute', 'row': 0, 'col': 0, 'label': 'Némítás', 'icon': 'mic', 'group': 'voice', 'available': true, 'lit_when': 'discord.mute', 'lit_label': 'Némítva', 'lit_icon': 'mic-off'},
        {'id': 'rec', 'row': 0, 'col': 1, 'label': 'Felvétel', 'icon': 'record', 'group': 'capture', 'available': false},
        {'id': 'vol', 'row': 1, 'col': 0, 'label': 'Rendszer', 'icon': 'volume', 'group': 'media', 'available': true, 'kind': 'slider', 'span': 2, 'value': 40},
      ],
    },
  ],
};

/// One pump hands the host's message to the app, the next draws the result.
Future<void> deliver(WidgetTester tester) async {
  await tester.pump();
  await tester.pump();
}

void main() {
  test('addresses become the host client endpoint', () {
    expect(clientUri('192.168.0.110').toString(), 'ws://192.168.0.110:8191/client');
    expect(clientUri(' 192.168.0.110:9000 ').toString(), 'ws://192.168.0.110:9000/client');
    expect(clientUri('http://asztali-pc:8191/').toString(), 'ws://asztali-pc:8191/client');
    expect(clientUri(''), isNull);
    expect(clientUri('   '), isNull);
  });

  test('pairs once, then reuses the token', () async {
    final store = MemoryStore();
    final channels = <FakeChannel>[];
    final uris = <Uri>[];
    DeckConnection connect() => DeckConnection(
      store: store,
      open: (uri) {
        uris.add(uri);
        channels.add(FakeChannel());
        return channels.last;
      },
    );

    final first = connect();
    expect(await first.connect('192.168.0.5'), isTrue);
    expect(uris.single.toString(), 'ws://192.168.0.5:8191/client');
    expect(channels.single.sent.single, {'type': 'hello', 'token': null, 'protocol': protocolVersion});

    channels.single.host({'type': 'pair_required'});
    await pumpEventQueue();
    expect(first.phase, Phase.pairing);

    first.pair(' 123456 ');
    expect(channels.single.sent.last, {'type': 'pair', 'code': '123456', 'name': 'Android telefon'});
    channels.single.host({'type': 'error', 'message': 'wrong pairing code'});
    await pumpEventQueue();
    expect(first.error, contains('Nem ez a kód'));

    channels.single.host({'type': 'paired', 'token': 'secret'});
    channels.single.host(deck);
    channels.single.host({'type': 'state', 'action': 'discord.mute', 'lit': true});
    await pumpEventQueue();
    expect(first.phase, Phase.ready);
    expect(first.error, isNull);
    expect(first.folders.single.keys.first.label, 'Némítás');
    expect(first.folders.single.keys.first.litIcon, 'mic-off');
    expect(first.states['discord.mute']!.lit, isTrue);
    first.dispose();

    // A fresh start finds the saved host and presents its token.
    final second = connect();
    await second.restore();
    expect(channels.last.sent.single['token'], 'secret');
    second.dispose();
  });

  test('refuses an address that cannot be one', () async {
    final connection = DeckConnection(store: MemoryStore(), open: (_) => fail('must not dial'));
    expect(await connection.connect('  '), isFalse);
    expect(connection.error, isNotNull);
    expect(connection.phase, Phase.idle);
  });

  testWidgets('shows the deck and sends presses', (tester) async {
    final channel = FakeChannel();
    final connection = DeckConnection(store: MemoryStore(), open: (_) => channel);
    await tester.pumpWidget(BestDeckApp(connection: connection));
    expect(find.text('Melyik géphez?'), findsOneWidget);

    await tester.enterText(find.byKey(const Key('address')), '192.168.0.5');
    await tester.tap(find.text('Csatlakozás'));
    await deliver(tester);
    channel.host({'type': 'pair_required'});
    await deliver(tester);
    expect(find.text('Párosító kód'), findsOneWidget);

    channel.host({'type': 'paired', 'token': 't'});
    channel.host(deck);
    await deliver(tester);
    expect(find.text('Gaming'), findsOneWidget);
    expect(find.text('Némítás'), findsOneWidget);

    await tester.tap(find.text('Némítás'));
    await deliver(tester);
    expect(channel.sent.last, {'type': 'press', 'key': 'mute'});

    // The key follows the state its plugin reports.
    channel.host({'type': 'state', 'action': 'discord.mute', 'lit': true});
    await deliver(tester);
    expect(find.text('Némítva'), findsOneWidget);

    // A key whose plugin is missing does nothing.
    final before = channel.sent.length;
    await tester.tap(find.text('Felvétel'));
    await deliver(tester);
    expect(channel.sent.length, before);

    // The slider shows the host's level, then follows the finger.
    expect(find.text('40'), findsOneWidget);
    final slider = find.text('Rendszer');
    final box = tester.getRect(find.ancestor(of: slider, matching: find.byType(GestureDetector)).first);
    await tester.dragFrom(box.centerLeft + const Offset(4, 0), Offset(box.width * 0.75, 0));
    await deliver(tester);
    final slides = channel.sent.where((m) => m['type'] == 'slide').toList();
    expect(slides, isNotEmpty);
    expect(slides.last['key'], 'vol');
    expect(slides.last['value'], inInclusiveRange(70, 80));

    // Another phone moved it: the level follows.
    channel.host({'type': 'level', 'key': 'vol', 'value': 12});
    await deliver(tester);
    expect(find.text('12'), findsOneWidget);

    await tester.pump(const Duration(milliseconds: 200));
  });
}
