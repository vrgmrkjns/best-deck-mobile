import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../connection.dart';
import '../theme.dart';

/// Choosing a host and, the first time, typing its pairing code.
class ConnectScreen extends StatefulWidget {
  const ConnectScreen({super.key, required this.connection});

  final DeckConnection connection;

  @override
  State<ConnectScreen> createState() => _ConnectScreenState();
}

class _ConnectScreenState extends State<ConnectScreen> {
  late final _address = TextEditingController(text: widget.connection.hostLabel);
  final _code = TextEditingController();

  @override
  void dispose() {
    _address.dispose();
    _code.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final connection = widget.connection;
    final pairing = connection.phase == Phase.pairing;
    const label = TextStyle(fontSize: 15, fontWeight: FontWeight.w500);
    const help = TextStyle(fontSize: 13, color: Palette.mute, height: 1.45);

    return Scaffold(
      body: SafeArea(
        child: ListView(
          padding: const EdgeInsets.fromLTRB(20, 32, 20, 24),
          children: [
            Text(
              pairing ? 'Párosítás' : 'Melyik géphez?',
              style: const TextStyle(fontSize: 30, fontWeight: FontWeight.w600, letterSpacing: -0.9),
            ),
            const SizedBox(height: 8),
            Text(
              pairing
                  ? 'Írd be a hat számjegyet, amit a gépen a best-deck szerkesztő Eszközök füle mutat. Egy gépet csak egyszer kell párosítani.'
                  : 'Indítsd el a best-decket a gépen, és írd be a címet, amit a szerkesztő Eszközök füle mutat. A telefon és a gép legyen ugyanazon a Wi-Fi-n.',
              style: const TextStyle(fontSize: 15, color: Palette.mute, height: 1.45),
            ),
            const SizedBox(height: 28),
            if (pairing) ...[
              const Text('Párosító kód', style: label),
              const SizedBox(height: 8),
              TextField(
                key: const Key('code'),
                controller: _code,
                autofocus: true,
                keyboardType: TextInputType.number,
                inputFormatters: [FilteringTextInputFormatter.digitsOnly, LengthLimitingTextInputFormatter(6)],
                textAlign: TextAlign.center,
                style: const TextStyle(fontSize: 30, letterSpacing: 12, fontFeatures: [FontFeature.tabularFigures()]),
                decoration: const InputDecoration(hintText: '000000'),
                onSubmitted: (_) => connection.pair(_code.text),
              ),
              const SizedBox(height: 12),
              FilledButton(onPressed: () => connection.pair(_code.text), child: const Text('Párosítás')),
              const SizedBox(height: 4),
              TextButton(onPressed: connection.forget, child: Text('Másik gép: ${connection.hostLabel}', style: const TextStyle(color: Palette.body))),
            ] else ...[
              const Text('A gép címe', style: label),
              const SizedBox(height: 8),
              TextField(
                key: const Key('address'),
                controller: _address,
                keyboardType: TextInputType.url,
                autocorrect: false,
                style: const TextStyle(fontSize: 16),
                decoration: const InputDecoration(hintText: '192.168.0.110:8191'),
                onSubmitted: (_) => connection.connect(_address.text),
              ),
              const SizedBox(height: 12),
              FilledButton(
                onPressed: connection.phase == Phase.connecting && connection.error == null ? null : () => connection.connect(_address.text),
                child: Text(connection.phase == Phase.connecting && connection.error == null ? 'Csatlakozás folyamatban' : 'Csatlakozás'),
              ),
            ],
            if (connection.error != null) ...[
              const SizedBox(height: 16),
              Semantics(
                liveRegion: true,
                child: Text(connection.error!, style: const TextStyle(fontSize: 14, color: Palette.alertInk, height: 1.45)),
              ),
            ],
            const SizedBox(height: 40),
            const Text('Nincs fiók és nincs felhő. A telefon csak a helyi hálózaton beszél a géppel.', style: help),
          ],
        ),
      ),
    );
  }
}
