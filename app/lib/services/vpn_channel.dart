import 'package:flutter/services.dart';

/// Dart side of the macOS NetworkExtension bridge (`macos/Runner/VpnChannel.swift`).
///
/// On macOS the TUN device lives in the `PacketTunnel` extension, so the app
/// drives connect/disconnect through `NETunnelProviderManager` over these
/// channels rather than the in-process Rust bridge.
class VpnChannel {
  static const _method = MethodChannel('netplane/vpn');
  static const _events = EventChannel('netplane/vpn/status');

  /// Path to the shared App Group container. Keys/auth files must live here so
  /// the extension process can read them.
  static Future<String> sharedContainerPath() async {
    final path = await _method.invokeMethod<String>('sharedContainerPath');
    if (path == null) {
      throw StateError('sharedContainerPath returned null');
    }
    return path;
  }

  /// Install/enable the VPN profile and start the tunnel. [config] is forwarded
  /// verbatim as the provider configuration.
  static Future<void> start(Map<String, dynamic> config) =>
      _method.invokeMethod<void>('start', config);

  /// Stop the active tunnel, if any.
  static Future<void> stop() => _method.invokeMethod<void>('stop');

  /// Stream of NEVPNStatus strings: connecting, connected, disconnected, etc.
  static Stream<String> statusStream() =>
      _events.receiveBroadcastStream().map((e) => e as String);
}
