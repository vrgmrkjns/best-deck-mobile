import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_svg/flutter_svg.dart';

import '../icons.dart';
import '../protocol.dart';
import '../theme.dart';

/// One key of the deck. Sinks the moment a finger lands; fires when it lifts.
class DeckKeyButton extends StatefulWidget {
  const DeckKeyButton({super.key, required this.deckKey, required this.state, required this.enabled, required this.onPressed});

  final DeckKey deckKey;

  /// The plugin state this key follows, if it has one.
  final KeyState? state;

  /// False while the link to the host is down.
  final bool enabled;
  final VoidCallback onPressed;

  @override
  State<DeckKeyButton> createState() => _DeckKeyButtonState();
}

class _DeckKeyButtonState extends State<DeckKeyButton> {
  bool _down = false;

  @override
  Widget build(BuildContext context) {
    final key = widget.deckKey;
    final lit = widget.state?.lit ?? false;
    final usable = widget.enabled && key.available;
    final group = Palette.group(key.group);
    final label = (lit ? widget.state?.label ?? key.litLabel : null) ?? key.label;
    final foreground = lit ? Colors.white : Palette.body;

    final face = DecoratedBox(
      decoration: BoxDecoration(
        borderRadius: BorderRadius.circular(14),
        border: Border.all(color: lit ? group : Palette.line),
        gradient: LinearGradient(
          begin: Alignment.topCenter,
          end: Alignment.bottomCenter,
          colors: lit
              ? [Color.alphaBlend(group.withValues(alpha: 0.46), Palette.keyTop), Color.alphaBlend(group.withValues(alpha: 0.32), Palette.keyBottom)]
              : const [Palette.keyTop, Palette.keyBottom],
        ),
      ),
      child: Padding(
        padding: const EdgeInsets.all(6),
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            SvgPicture.string(iconSvg((lit ? key.litIcon : null) ?? key.icon, Palette.hex(lit ? Colors.white : group)), width: 28, height: 28),
            const SizedBox(height: 8),
            Text(
              label,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: TextStyle(fontSize: 12, fontWeight: lit ? FontWeight.w600 : FontWeight.w500, color: foreground),
            ),
          ],
        ),
      ),
    );

    return Semantics(
      button: true,
      enabled: usable,
      toggled: key.litWhen == null ? null : lit,
      label: label,
      excludeSemantics: true,
      child: GestureDetector(
        behavior: HitTestBehavior.opaque,
        onTapDown: usable ? (_) => setState(() => _down = true) : null,
        onTapCancel: usable ? () => setState(() => _down = false) : null,
        onTapUp: usable
            ? (_) {
                setState(() => _down = false);
                HapticFeedback.lightImpact();
                widget.onPressed();
              }
            : null,
        child: AnimatedScale(
          scale: _down ? 0.95 : 1,
          duration: MediaQuery.disableAnimationsOf(context) ? Duration.zero : const Duration(milliseconds: 120),
          curve: Curves.easeOutCubic,
          child: Opacity(opacity: usable ? 1 : 0.42, child: face),
        ),
      ),
    );
  }
}
