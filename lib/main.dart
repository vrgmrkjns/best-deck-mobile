import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import 'connection.dart';
import 'screens/connect_screen.dart';
import 'screens/deck_screen.dart';
import 'theme.dart';

void main() {
  WidgetsFlutterBinding.ensureInitialized();
  SystemChrome.setSystemUIOverlayStyle(
    const SystemUiOverlayStyle(statusBarColor: Colors.transparent, systemNavigationBarColor: Palette.canvas),
  );
  runApp(BestDeckApp(connection: DeckConnection(store: PreferencesStore())..restore()));
}

class BestDeckApp extends StatelessWidget {
  const BestDeckApp({super.key, required this.connection});

  final DeckConnection connection;

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'best-deck',
      debugShowCheckedModeBanner: false,
      theme: deckTheme(),
      home: ListenableBuilder(
        listenable: connection,
        builder: (context, _) =>
            connection.phase == Phase.ready ? DeckScreen(connection: connection) : ConnectScreen(connection: connection),
      ),
    );
  }
}
