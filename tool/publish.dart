// Builds the release APK and publishes it to the best-deck host on this PC, which then
// offers it to every phone running an older build.
//
//   dart run tool/publish.dart [--bump] [--to <folder>]
//
// --bump  raises the build number in pubspec.yaml first (each release needs a higher one).
// --to    the host's updates folder; defaults to %APPDATA%\best-deck\updates.
import 'dart:convert';
import 'dart:io';

import 'package:crypto/crypto.dart';

Future<void> main(List<String> args) async {
  final pubspec = File('pubspec.yaml');
  var text = await pubspec.readAsString();
  final match = RegExp(r'^version: (\d+\.\d+\.\d+)\+(\d+)\s*$', multiLine: true).firstMatch(text);
  if (match == null) fail('pubspec.yaml needs a version like 1.2.3+4.');
  final name = match.group(1)!;
  var code = int.parse(match.group(2)!);

  final target = Directory(_option(args, '--to') ?? '${Platform.environment['APPDATA']}\\best-deck\\updates');
  final description = File('${target.path}${Platform.pathSeparator}best-deck.json');
  final published = description.existsSync() ? (jsonDecode(description.readAsStringSync())['version_code'] as int? ?? 0) : 0;

  if (args.contains('--bump')) {
    code = (code > published ? code : published) + 1;
    text = text.replaceFirst(match.group(0)!, 'version: $name+$code');
    await pubspec.writeAsString(text);
    stdout.writeln('Build number raised to $code.');
  }
  if (code <= published) {
    fail('Build $code is not newer than the published build $published. Raise it in pubspec.yaml or pass --bump.');
  }
  if (!File('android/key.properties').existsSync()) {
    fail('android/key.properties is missing, so the APK would be signed with the debug key and phones could not update to it.');
  }

  final flutter = Platform.isWindows ? 'flutter.bat' : 'flutter';
  final build = await Process.start(flutter, ['build', 'apk', '--release'], mode: ProcessStartMode.inheritStdio, runInShell: true);
  if (await build.exitCode != 0) fail('flutter build apk failed.');

  final apk = File('build/app/outputs/flutter-apk/app-release.apk');
  final bytes = await apk.readAsBytes();
  await target.create(recursive: true);
  // The APK first and the description last, so the host never offers a half-copied file.
  await apk.copy('${target.path}${Platform.pathSeparator}best-deck.apk');
  await description.writeAsString(
    const JsonEncoder.withIndent('  ').convert({'version_code': code, 'version_name': name, 'sha256': sha256.convert(bytes).toString()}),
  );
  stdout.writeln('Published $name (build $code) to ${target.path}.');
}

String? _option(List<String> args, String name) {
  final index = args.indexOf(name);
  return index >= 0 && index + 1 < args.length ? args[index + 1] : null;
}

Never fail(String message) {
  stderr.writeln(message);
  exit(1);
}
