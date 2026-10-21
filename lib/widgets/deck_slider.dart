import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../protocol.dart';
import '../theme.dart';

/// A volume tile that is the slider itself: the level follows the finger 1:1.
class DeckSlider extends StatefulWidget {
  const DeckSlider({super.key, required this.deckKey, required this.level, required this.enabled, required this.onChanged});

  final DeckKey deckKey;

  /// The host's level, 0 to 100, if known.
  final int? level;

  /// False while the link to the host is down.
  final bool enabled;
  final ValueChanged<int> onChanged;

  @override
  State<DeckSlider> createState() => _DeckSliderState();
}

class _DeckSliderState extends State<DeckSlider> {
  /// While a finger is down this wins over whatever the host reports.
  int? _dragging;
  DateTime _lastSent = DateTime.fromMillisecondsSinceEpoch(0);

  int get _shown => _dragging ?? widget.level ?? 0;

  void _moveTo(double x, double width, {bool last = false}) {
    final value = (x / width * 100).round().clamp(0, 100);
    if (value != _dragging && value % 10 == 0) HapticFeedback.selectionClick();
    setState(() => _dragging = value);
    // About twenty updates a second feels live without flooding the host.
    final now = DateTime.now();
    if (last || now.difference(_lastSent) >= const Duration(milliseconds: 50)) {
      _lastSent = now;
      widget.onChanged(value);
    }
  }

  void _end() {
    if (_dragging != null) widget.onChanged(_dragging!);
    setState(() => _dragging = null);
  }

  void _step(int delta) => widget.onChanged((_shown + delta).clamp(0, 100));

  @override
  Widget build(BuildContext context) {
    final key = widget.deckKey;
    final group = Palette.group(key.group.isEmpty ? 'media' : key.group);
    final known = widget.level != null || _dragging != null;

    return Semantics(
      slider: true,
      label: key.label,
      value: known ? '$_shown%' : 'ismeretlen',
      increasedValue: '${(_shown + 5).clamp(0, 100)}%',
      decreasedValue: '${(_shown - 5).clamp(0, 100)}%',
      enabled: widget.enabled,
      onIncrease: widget.enabled ? () => _step(5) : null,
      onDecrease: widget.enabled ? () => _step(-5) : null,
      excludeSemantics: true,
      child: LayoutBuilder(
        builder: (context, constraints) {
          final width = constraints.maxWidth;
          return GestureDetector(
            behavior: HitTestBehavior.opaque,
            // Horizontal only, so a vertical swipe still scrolls a tall deck.
            onHorizontalDragStart: widget.enabled ? (details) => _moveTo(details.localPosition.dx, width) : null,
            onHorizontalDragUpdate: widget.enabled ? (details) => _moveTo(details.localPosition.dx, width) : null,
            onHorizontalDragEnd: widget.enabled ? (_) => _end() : null,
            onHorizontalDragCancel: widget.enabled ? _end : null,
            onTapUp: widget.enabled
                ? (details) {
                    _moveTo(details.localPosition.dx, width, last: true);
                    _end();
                  }
                : null,
            child: Opacity(
              opacity: widget.enabled ? 1 : 0.42,
              child: ClipRRect(
                borderRadius: BorderRadius.circular(14),
                child: DecoratedBox(
                  decoration: BoxDecoration(
                    color: Palette.keyBottom,
                    border: Border.all(color: _dragging != null ? group : Palette.line),
                    borderRadius: BorderRadius.circular(14),
                  ),
                  child: Stack(
                    fit: StackFit.expand,
                    children: [
                      FractionallySizedBox(
                        alignment: Alignment.centerLeft,
                        widthFactor: _shown / 100,
                        child: ColoredBox(color: Color.alphaBlend(group.withValues(alpha: 0.28), Palette.keyBottom)),
                      ),
                      Align(
                        alignment: Alignment(_shown / 50 - 1, 0),
                        child: Container(width: 4, height: 40, decoration: BoxDecoration(color: Palette.ink, borderRadius: BorderRadius.circular(2))),
                      ),
                      Padding(
                        padding: const EdgeInsets.fromLTRB(14, 10, 14, 10),
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          mainAxisAlignment: MainAxisAlignment.spaceBetween,
                          children: [
                            Text(key.label, maxLines: 1, overflow: TextOverflow.ellipsis, style: const TextStyle(fontSize: 12, fontWeight: FontWeight.w500, color: Palette.body)),
                            Text(
                              known ? '$_shown' : '-',
                              style: const TextStyle(fontSize: 22, fontWeight: FontWeight.w500, color: Palette.ink, fontFeatures: [FontFeature.tabularFigures()]),
                            ),
                          ],
                        ),
                      ),
                    ],
                  ),
                ),
              ),
            ),
          );
        },
      ),
    );
  }
}
