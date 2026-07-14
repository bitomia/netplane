import 'dart:convert';
import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:path_provider/path_provider.dart';

/// The connection details the user typed on the login screen, so they don't
/// have to be re-entered after a disconnect.
@immutable
class ConnectionSettings {
  const ConnectionSettings({
    required this.host,
    required this.linkCode,
    required this.transport,
  });

  final String host;
  final String linkCode;

  /// Transport name as stored by [Transport.name] ("websocket" | "udp").
  final String transport;

  Map<String, dynamic> toJson() => {
    'host': host,
    'link_code': linkCode,
    'transport': transport,
  };

  static ConnectionSettings? fromJson(Map<String, dynamic> json) {
    final host = json['host'];
    final linkCode = json['link_code'];
    final transport = json['transport'];
    if (host is! String || linkCode is! String) return null;
    return ConnectionSettings(
      host: host,
      linkCode: linkCode,
      transport: transport is String ? transport : 'websocket',
    );
  }
}

/// Persists the last successful connection details next to the key files.
///
/// Stored as plain JSON in the application support directory: the link code is
/// no more sensitive than the auth key already written there.
class ConnectionStore {
  ConnectionStore._();
  static final ConnectionStore instance = ConnectionStore._();

  static const _fileName = 'connection.json';

  ConnectionSettings? _cached;

  Future<File> _file() async {
    final dir = await getApplicationSupportDirectory();
    return File('${dir.path}/$_fileName');
  }

  /// The last saved settings, or null if nothing was saved yet or the file is
  /// unreadable/corrupt.
  Future<ConnectionSettings?> load() async {
    if (_cached != null) return _cached;
    try {
      final file = await _file();
      if (!await file.exists()) return null;
      final json = jsonDecode(await file.readAsString());
      if (json is! Map<String, dynamic>) return null;
      return _cached = ConnectionSettings.fromJson(json);
    } catch (e) {
      if (kDebugMode) print('connection store: load failed: $e');
      return null;
    }
  }

  Future<void> save(ConnectionSettings settings) async {
    _cached = settings;
    try {
      final file = await _file();
      await file.parent.create(recursive: true);
      await file.writeAsString(jsonEncode(settings.toJson()));
    } catch (e) {
      if (kDebugMode) print('connection store: save failed: $e');
    }
  }

  /// Forget the stored details (e.g. when the user wants to sign in elsewhere).
  Future<void> clear() async {
    _cached = null;
    try {
      final file = await _file();
      if (await file.exists()) await file.delete();
    } catch (e) {
      if (kDebugMode) print('connection store: clear failed: $e');
    }
  }
}
