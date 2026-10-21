/// Messages of the host's `/client` WebSocket endpoint, as documented in the
/// best-deck-windows README.
library;

/// Sent in `hello` so a host can refuse a client it no longer understands.
const protocolVersion = 1;

class DeckKey {
  const DeckKey({
    required this.id,
    required this.row,
    required this.col,
    required this.label,
    required this.icon,
    required this.group,
    required this.available,
    this.kind = 'button',
    this.span = 1,
    this.value,
    this.litWhen,
    this.litLabel,
    this.litIcon,
  });

  final String id;
  final int row;
  final int col;
  final String label;
  final String icon;

  /// Colour group: voice, capture, media, game or sys.
  final String group;

  /// False while a plugin this key needs is not connected on the host.
  final bool available;

  /// `button` or `slider`.
  final String kind;

  /// How many grid cells the key covers to the right.
  final int span;

  /// A slider's level (0 to 100) when the host could read it.
  final int? value;

  bool get isSlider => kind == 'slider';

  /// The `plugin.action` whose state lights this key.
  final String? litWhen;
  final String? litLabel;

  /// Icon while lit, such as a crossed-out microphone; the normal icon otherwise.
  final String? litIcon;

  factory DeckKey.fromJson(Map<String, dynamic> json) => DeckKey(
    id: json['id'] as String,
    row: json['row'] as int,
    col: json['col'] as int,
    label: json['label'] as String? ?? '',
    icon: json['icon'] as String? ?? '',
    group: json['group'] as String? ?? '',
    available: json['available'] as bool? ?? true,
    kind: json['kind'] as String? ?? 'button',
    span: json['span'] as int? ?? 1,
    value: json['value'] as int?,
    litWhen: json['lit_when'] as String?,
    litLabel: json['lit_label'] as String?,
    litIcon: json['lit_icon'] as String?,
  );
}

class DeckFolder {
  const DeckFolder({required this.name, required this.columns, required this.rows, required this.keys});

  final String name;
  final int columns;
  final int rows;
  final List<DeckKey> keys;

  factory DeckFolder.fromJson(Map<String, dynamic> json) => DeckFolder(
    name: json['name'] as String? ?? '',
    columns: json['columns'] as int,
    rows: json['rows'] as int,
    keys: [for (final key in json['keys'] as List) DeckKey.fromJson(key as Map<String, dynamic>)],
  );
}

class KeyState {
  const KeyState({required this.lit, this.label});

  final bool lit;
  final String? label;
}

/// Turns what the user typed (`192.168.0.110`, `192.168.0.110:8191`,
/// `http://pc:8191/`) into the host's WebSocket address, or null if it cannot be one.
Uri? clientUri(String input) {
  var text = input.trim();
  if (text.isEmpty) return null;
  text = text.replaceFirst(RegExp(r'^[a-zA-Z]+://'), '');
  final uri = Uri.tryParse('ws://$text');
  if (uri == null || uri.host.isEmpty) return null;
  return Uri(scheme: 'ws', host: uri.host, port: uri.hasPort ? uri.port : 8191, path: '/client');
}
