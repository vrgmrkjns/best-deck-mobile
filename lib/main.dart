import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import 'connection.dart';
import 'screens/connect_screen.dart';
import 'screens/deck_screen.dart';
import 'theme.dart';
import 'updater.dart';

void main() {
  WidgetsFlutterBinding.ensureInitialized();
  SystemChrome.setSystemUIOverlayStyle(
    const SystemUiOverlayStyle(statusBarColor: Colors.transparent, systemNavigationBarColor: Palette.canvas),
  );
  final connection = DeckConnection(store: PreferencesStore())..restore();
  final updater = Updater();
  // Ask the host for a newer app each time the link comes up.
  var wasOnline = false;
  connection.addListener(() {
    final online = connection.online && connection.phase == Phase.ready;
    if (online && !wasOnline && Updater.supported) updater.check(connection.uri!);
    wasOnline = online;
  });
  runApp(BestDeckApp(connection: connection, updater: updater));
}

class BestDeckApp extends StatelessWidget {
  const BestDeckApp({super.key, required this.connection, this.updater});

  final DeckConnection connection;
  final Updater? updater;

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'best-deck',
      debugShowCheckedModeBanner: false,
      theme: deckTheme(),
      home: ListenableBuilder(
        listenable: connection,
        builder: (context, _) =>
            connection.phase == Phase.ready ? DeckScreen(connection: connection, updater: updater) : ConnectScreen(connection: connection),
      ),
    );
  }
}
