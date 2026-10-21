import 'package:flutter/material.dart';

/// The deck's palette: monochrome chrome, colour only on the keys.
abstract final class Palette {
  static const canvas = Color(0xFF0A0B0D);
  static const surface = Color(0xFF111214);
  static const keyTop = Color(0xFF1C1E21);
  static const keyBottom = Color(0xFF141518);
  static const line = Color(0xFF26282C);
  static const lineStrong = Color(0xFF33363B);
  static const ink = Color(0xFFF2F3F5);
  static const body = Color(0xFFC9CBD1);
  static const mute = Color(0xFF9A9CA3);
  static const online = Color(0xFF4CC38A);
  static const alertSurface = Color(0xFF2A1516);
  static const alertInk = Color(0xFFFFB4B4);

  static const _groups = <String, Color>{
    'voice': Color(0xFF6E9BFF),
    'capture': Color(0xFFFF6B6B),
    'media': Color(0xFF4CC38A),
    'game': Color(0xFFF2B23E),
    'sys': Color(0xFF9A9CA3),
  };

  static Color group(String name) => _groups[name] ?? _groups['sys']!;

  /// `#RRGGBB` for embedding in an SVG string.
  static String hex(Color color) => '#${(color.toARGB32() & 0xFFFFFF).toRadixString(16).padLeft(6, '0')}';
}

ThemeData deckTheme() {
  final base = ThemeData(brightness: Brightness.dark, useMaterial3: true);
  OutlineInputBorder border(Color color) =>
      OutlineInputBorder(borderRadius: BorderRadius.circular(12), borderSide: BorderSide(color: color));
  return base.copyWith(
    scaffoldBackgroundColor: Palette.canvas,
    colorScheme: base.colorScheme.copyWith(
      surface: Palette.canvas,
      primary: Palette.ink,
      onPrimary: Palette.canvas,
      onSurface: Palette.ink,
      error: Palette.alertInk,
    ),
    textTheme: base.textTheme.apply(bodyColor: Palette.ink, displayColor: Palette.ink),
    inputDecorationTheme: InputDecorationTheme(
      filled: true,
      fillColor: Palette.surface,
      hintStyle: const TextStyle(color: Color(0xFF6E7078)),
      contentPadding: const EdgeInsets.symmetric(horizontal: 14, vertical: 14),
      enabledBorder: border(Palette.line),
      focusedBorder: border(Palette.ink),
    ),
    filledButtonTheme: FilledButtonThemeData(
      style: FilledButton.styleFrom(
        backgroundColor: Colors.white,
        foregroundColor: Palette.canvas,
        minimumSize: const Size(0, 48),
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(10)),
        textStyle: const TextStyle(fontSize: 15, fontWeight: FontWeight.w600),
      ),
    ),
    textButtonTheme: TextButtonThemeData(
      style: TextButton.styleFrom(foregroundColor: Palette.ink, minimumSize: const Size(0, 44)),
    ),
  );
}
