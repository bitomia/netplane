import 'dart:io' show Platform;

import 'package:path_provider/path_provider.dart';

import '../src/rust/api/client.dart' as rust;
import 'vpn_channel.dart';

/// High-level facade over the connect → authenticate → run flow.
///
/// Keygen and authentication always run in-process via the Rust bridge
/// (`src/rust/api/client.dart`). Opening the tunnel is platform-specific:
///   - macOS: driven through the NetworkExtension `PacketTunnel` extension via
///     [VpnChannel] (a sandboxed app can't create a utun itself).
///   - other platforms: the in-process Rust `connect` path.
class NetplaneService {
  NetplaneService._();
  static final NetplaneService instance = NetplaneService._();

  /// macOS auto-assigns the utun name (the value is ignored there); Linux uses it.
  String get _defaultTunDev => Platform.isMacOS ? 'utun' : 'tun0';

  /// Directory that holds the key/auth files. On macOS this must be the shared
  /// App Group container so the extension process can read the same files.
  Future<String> _keyDir() async {
    if (Platform.isMacOS) {
      return VpnChannel.sharedContainerPath();
    }
    final dir = await getApplicationSupportDirectory();
    return dir.path;
  }

  Future<rust.NetplaneConfig> _buildConfig({
    required String host,
    required String linkCode,
    required String transport,
  }) async {
    final base = await _keyDir();
    String path(String name) => '$base/$name';

    return rust.NetplaneConfig(
      host: host,
      port: 0, // 0 => client default (5000)
      transport: transport,
      linkCode: linkCode,
      tunDev: _defaultTunDev,
      authkeyPath: path('auth.key'),
      publicKeyPath: path('public.key'),
      privateKeyPath: path('private.key'),
      authPort: 0, // 0 => client default (8000)
      loopbackRelay: false,
      noEncryption: false,
    );
  }

  /// Ensure keys exist, exchange the link code for an auth key, then open the
  /// tunnel. Returns a status stream of [rust.ConnectionEvent]. Throws if key
  /// generation or authentication fails (before the tunnel is attempted).
  Future<Stream<rust.ConnectionEvent>> startConnection({
    required String host,
    required String linkCode,
    required String transport,
  }) async {
    final config = await _buildConfig(
      host: host,
      linkCode: linkCode,
      transport: transport,
    );

    await rust.generateKeys(
      publicKeyPath: config.publicKeyPath,
      privateKeyPath: config.privateKeyPath,
    );
    await rust.authenticate(config: config);
    if (Platform.isMacOS) {
      await VpnChannel.start({
        'host': config.host,
        'port': config.port,
        'transport': config.transport,
        'authkey_path': config.authkeyPath,
        'public_key_path': config.publicKeyPath,
        'private_key_path': config.privateKeyPath,
        'loopback_relay': config.loopbackRelay,
        'no_encryption': config.noEncryption,
      });
      return VpnChannel.statusStream()
          .map(_mapStatus)
          .where((e) => e != null)
          .cast<rust.ConnectionEvent>();
    }

    return rust.connect(config: config);
  }

  /// Map NEVPNStatus strings to the existing [rust.ConnectionEvent] variants so
  /// the UI is platform-agnostic. Returns null for transient states to skip.
  rust.ConnectionEvent? _mapStatus(String status) {
    switch (status) {
      case 'connecting':
      case 'reconnecting':
        return const rust.ConnectionEvent.connecting();
      case 'connected':
        // IP/netmask are assigned inside the extension; the UI doesn't use them.
        return const rust.ConnectionEvent.connected(ipAddr: '', netmask: '');
      case 'disconnected':
        return const rust.ConnectionEvent.disconnected();
      case 'invalid':
        return const rust.ConnectionEvent.error('VPN configuration invalid');
      default: // disconnecting, unknown
        return null;
    }
  }

  /// Cancel the active connection, if any.
  Future<void> stop() {
    if (Platform.isMacOS) {
      return VpnChannel.stop();
    }
    return rust.disconnect();
  }
}
