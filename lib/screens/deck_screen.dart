import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:wakelock_plus/wakelock_plus.dart';

import '../connection.dart';
import '../protocol.dart';
import '../theme.dart';
import '../updater.dart';
import '../widgets/deck_key.dart';
import '../widgets/deck_slider.dart';

class DeckScreen extends StatefulWidget {
  const DeckScreen({super.key, required this.connection, this.updater});

  final DeckConnection connection;
  final Updater? updater;

  @override
  State<DeckScreen> createState() => _DeckScreenState();
}

class _DeckScreenState extends State<DeckScreen> {
  int _folder = 0;

  @override
  void initState() {
    super.initState();
    // A deck that dims after thirty seconds is useless next to a keyboard.
    WakelockPlus.enable().catchError((_) {});
  }

  @override
  void dispose() {
    WakelockPlus.disable().catchError((_) {});
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final connection = widget.connection;
    final folders = connection.folders;
    final index = folders.isEmpty ? 0 : _folder.clamp(0, folders.length - 1);
    final notice = !connection.online ? 'Nincs kapcsolat a géppel. Újrapróbálom.' : connection.error;

    return Scaffold(
      body: SafeArea(
        child: Padding(
          padding: const EdgeInsets.fromLTRB(16, 12, 16, 12),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Row(
                children: [
                  Expanded(
                    child: Text(
                      connection.profile,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: const TextStyle(fontSize: 24, fontWeight: FontWeight.w600, letterSpacing: -0.6),
                    ),
                  ),
                  TextButton.icon(
                    onPressed: () => _showHost(context),
                    icon: Container(
                      width: 8,
                      height: 8,
                      decoration: BoxDecoration(shape: BoxShape.circle, color: connection.online ? Palette.online : Palette.alertInk),
                    ),
                    label: Text(connection.uri?.host ?? '', style: const TextStyle(fontSize: 13, color: Palette.body)),
                  ),
                ],
              ),
              if (folders.length > 1) _FolderTabs(folders: folders, selected: index, onSelected: (i) => setState(() => _folder = i)),
              const SizedBox(height: 12),
              if (widget.updater case final updater?)
                ListenableBuilder(
                  listenable: updater,
                  builder: (context, _) => updater.available == null
                      ? const SizedBox.shrink()
                      : Padding(padding: const EdgeInsets.only(bottom: 12), child: _UpdateBanner(updater: updater)),
                ),
              if (notice != null) ...[_Notice(text: notice), const SizedBox(height: 12)],
              Expanded(
                child: folders.isEmpty
                    ? const Center(
                        child: Text('Ez a deck még üres. Adj hozzá gombokat a gépen, a szerkesztőben.', textAlign: TextAlign.center, style: TextStyle(color: Palette.mute)),
                      )
                    : _Grid(folder: folders[index], connection: connection),
              ),
            ],
          ),
        ),
      ),
    );
  }

  void _showHost(BuildContext context) {
    final connection = widget.connection;
    showModalBottomSheet<void>(
      context: context,
      backgroundColor: Palette.surface,
      showDragHandle: true,
      builder: (sheet) => SafeArea(
        child: Padding(
          padding: const EdgeInsets.fromLTRB(20, 0, 20, 20),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Text(connection.hostLabel, style: const TextStyle(fontSize: 17, fontWeight: FontWeight.w600)),
              const SizedBox(height: 4),
              Text(connection.online ? 'Csatlakozva' : 'Nincs kapcsolat', style: const TextStyle(color: Palette.mute)),
              const SizedBox(height: 20),
              OutlinedButton(
                style: OutlinedButton.styleFrom(
                  foregroundColor: Palette.ink,
                  minimumSize: const Size(0, 48),
                  side: const BorderSide(color: Palette.lineStrong),
                  shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(10)),
                ),
                onPressed: () {
                  Navigator.pop(sheet);
                  connection.forget();
                },
                child: const Text('Másik gép választása'),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _FolderTabs extends StatelessWidget {
  const _FolderTabs({required this.folders, required this.selected, required this.onSelected});

  final List<DeckFolder> folders;
  final int selected;
  final ValueChanged<int> onSelected;

  @override
  Widget build(BuildContext context) {
    return DecoratedBox(
      decoration: const BoxDecoration(border: Border(bottom: BorderSide(color: Color(0xFF1E2023)))),
      child: SingleChildScrollView(
        scrollDirection: Axis.horizontal,
        child: Row(
          children: [
            for (final (i, folder) in folders.indexed)
              Semantics(
                selected: i == selected,
                button: true,
                child: InkWell(
                  onTap: () => onSelected(i),
                  child: Container(
                    constraints: const BoxConstraints(minHeight: 44),
                    margin: const EdgeInsets.only(right: 22),
                    alignment: Alignment.center,
                    decoration: BoxDecoration(
                      border: Border(bottom: BorderSide(width: 2, color: i == selected ? Palette.ink : Colors.transparent)),
                    ),
                    child: Text(
                      folder.name,
                      style: TextStyle(fontSize: 15, fontWeight: i == selected ? FontWeight.w500 : FontWeight.w400, color: i == selected ? Palette.ink : Palette.mute),
                    ),
                  ),
                ),
              ),
          ],
        ),
      ),
    );
  }
}

class _UpdateBanner extends StatelessWidget {
  const _UpdateBanner({required this.updater});

  final Updater updater;

  @override
  Widget build(BuildContext context) {
    final progress = updater.progress;
    return Semantics(
      liveRegion: true,
      child: Container(
        padding: const EdgeInsets.fromLTRB(14, 10, 8, 10),
        decoration: BoxDecoration(color: Palette.surface, border: Border.all(color: Palette.line), borderRadius: BorderRadius.circular(12)),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Row(
              children: [
                Expanded(
                  child: Text(
                    progress == null ? 'Új verzió a gépen: ${updater.available!.versionName}' : 'Letöltés: ${(progress * 100).round()}%',
                    style: const TextStyle(fontSize: 14, fontWeight: FontWeight.w500),
                  ),
                ),
                TextButton(
                  onPressed: progress == null ? updater.start : null,
                  child: const Text('Frissítés', style: TextStyle(fontWeight: FontWeight.w600)),
                ),
              ],
            ),
            if (progress != null)
              Padding(
                padding: const EdgeInsets.only(right: 6, bottom: 4),
                child: LinearProgressIndicator(value: progress, minHeight: 3, color: Palette.ink, backgroundColor: Palette.line),
              ),
            if (updater.error != null)
              Padding(
                padding: const EdgeInsets.only(right: 6, bottom: 2),
                child: Text(updater.error!, style: const TextStyle(fontSize: 13, color: Palette.alertInk)),
              ),
          ],
        ),
      ),
    );
  }
}

class _Notice extends StatelessWidget {
  const _Notice({required this.text});

  final String text;

  @override
  Widget build(BuildContext context) {
    return Semantics(
      liveRegion: true,
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 12),
        decoration: BoxDecoration(color: Palette.alertSurface, borderRadius: BorderRadius.circular(12)),
        child: Text(text, style: const TextStyle(fontSize: 13, color: Palette.alertInk)),
      ),
    );
  }
}

/// Lays the keys out on the folder's grid, sized so every row fits without scrolling when it can.
class _Grid extends StatelessWidget {
  const _Grid({required this.folder, required this.connection});

  static const _gap = 10.0;

  final DeckFolder folder;
  final DeckConnection connection;

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraints) {
        final cellWidth = (constraints.maxWidth - _gap * (folder.columns - 1)) / folder.columns;
        final fitted = (constraints.maxHeight - _gap * (folder.rows - 1)) / folder.rows;
        // Never squeeze a key below what its icon and label need (about 66px); scroll instead.
        final cellHeight = math.max(72.0, math.min(fitted, cellWidth * 1.15));
        final height = cellHeight * folder.rows + _gap * (folder.rows - 1);
        return SingleChildScrollView(
          child: SizedBox(
            height: height,
            child: Stack(
              children: [
                for (final key in folder.keys)
                  Positioned(
                    left: key.col * (cellWidth + _gap),
                    top: key.row * (cellHeight + _gap),
                    width: cellWidth * key.span + _gap * (key.span - 1),
                    height: cellHeight,
                    child: key.isSlider
                        ? DeckSlider(
                            deckKey: key,
                            level: connection.levels[key.id],
                            enabled: connection.online,
                            onChanged: (value) => connection.slide(key.id, value),
                          )
                        : DeckKeyButton(
                            deckKey: key,
                            state: key.litWhen == null ? null : connection.states[key.litWhen],
                            enabled: connection.online,
                            onPressed: () => connection.press(key.id),
                          ),
                  ),
              ],
            ),
          ),
        );
      },
    );
  }
}
