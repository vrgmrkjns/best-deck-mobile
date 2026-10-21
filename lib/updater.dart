import 'dart:async';
import 'dart:convert';

import 'package:flutter/foundation.dart';
import 'package:http/http.dart' as http;
import 'package:ota_update/ota_update.dart';
import 'package:package_info_plus/package_info_plus.dart';

/// A phone app build the host offers, from its `/app/version`.
class AppRelease {
  const AppRelease({required this.versionCode, required this.versionName, required this.sha256});

  final int versionCode;
  final String versionName;
  final String sha256;

  static AppRelease? fromJson(Object? json) {
    if (json is! Map<String, dynamic>) return null;
    final code = json['version_code'], name = json['version_name'], sha = json['sha256'];
    if (code is! int || name is! String || sha is! String) return null;
    return AppRelease(versionCode: code, versionName: name, sha256: sha);
  }
}

/// Downloads [url], checks it against [sha256] and hands it to the Android installer.
/// Yields download progress from 0 to 1; completes once the installer is showing.
typedef Installer = Stream<double> Function(Uri url, String sha256);

Stream<double> otaInstaller(Uri url, String sha256) async* {
  await for (final event in OtaUpdate().execute(url.toString(), destinationFilename: 'best-deck.apk', sha256checksum: sha256)) {
    switch (event.status) {
      case OtaStatus.DOWNLOADING:
        yield (double.tryParse(event.value ?? '') ?? 0) / 100;
      case OtaStatus.INSTALLING || OtaStatus.INSTALLATION_DONE:
        return;
      case OtaStatus.CHECKSUM_ERROR:
        throw 'A letöltött fájl sérült. Próbáld újra.';
      case OtaStatus.PERMISSION_NOT_GRANTED_ERROR:
        throw 'Engedélyezd a telepítést ebből az alkalmazásból, aztán próbáld újra.';
      default:
        throw 'A frissítés nem sikerült: ${event.value ?? event.status.name}';
    }
  }
}

Future<int> installedBuild() async => int.tryParse((await PackageInfo.fromPlatform()).buildNumber) ?? 0;

/// Asks the host whether it has a newer build of this app, and installs it on request.
class Updater extends ChangeNotifier {
  Updater({http.Client? client, Future<int> Function()? currentBuild, Installer? install})
    : _client = client ?? http.Client(),
      _currentBuild = currentBuild ?? installedBuild,
      _install = install ?? otaInstaller;

  final http.Client _client;
  final Future<int> Function() _currentBuild;
  final Installer _install;

  /// A newer build the host offers, if any.
  AppRelease? available;

  /// Download progress from 0 to 1 while updating.
  double? progress;
  String? error;
  Uri? _host;

  /// Only Android can install an APK; elsewhere there is nothing to offer.
  static bool get supported => !kIsWeb && defaultTargetPlatform == TargetPlatform.android;

  /// [clientUri] is the host's WebSocket address; the update lives on the same host and port.
  Future<void> check(Uri clientUri) async {
    final host = clientUri.replace(scheme: 'http', path: '');
    try {
      final response = await _client.get(host.replace(path: '/app/version')).timeout(const Duration(seconds: 5));
      final release = response.statusCode == 200 ? AppRelease.fromJson(jsonDecode(response.body)) : null;
      final newer = release != null && release.versionCode > await _currentBuild();
      _host = host;
      available = newer ? release : null;
    } catch (_) {
      // An older host without updates, or a flaky link: nothing to offer this time.
      available = null;
    }
    notifyListeners();
  }

  Future<void> start() async {
    final release = available, host = _host;
    if (release == null || host == null || progress != null) return;
    error = null;
    progress = 0;
    notifyListeners();
    try {
      await for (final value in _install(host.replace(path: '/app/best-deck.apk'), release.sha256)) {
        progress = value;
        notifyListeners();
      }
    } catch (failure) {
      error = '$failure';
    }
    progress = null;
    notifyListeners();
  }

  @override
  void dispose() {
    _client.close();
    super.dispose();
  }
}
