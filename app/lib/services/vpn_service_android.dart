import 'package:flutter/services.dart';

/// Dart side of the Android `VpnService` bridge
/// (`android/.../NetplaneVpnService.kt` + `MainActivity.kt`).
///
/// On Android only a `VpnService` may create a TUN device, and the tunnel IP
/// must be known before `establish()`. So the connect flow is split: the Rust
/// bridge runs the handshake ([rust.prepareTunnel]) to learn the IP/netmask,
/// this channel establishes the tunnel and returns the raw fd, and the Rust
/// bridge then runs the packet loop on that fd ([rust.connectFd]).
class AndroidVpnChannel {
  static const _method = MethodChannel('netplane/vpn/android');

  /// Request VPN consent from the user. Returns true once the tunnel is allowed
  /// (immediately if consent was already granted). Throws on channel errors.
  static Future<bool> prepare() async {
    final ok = await _method.invokeMethod<bool>('prepare');
    return ok ?? false;
  }

  /// Build the tunnel for [ipAddr]/[netmask] and return the TUN file descriptor.
  /// The fd stays owned by the service; pass it to `rust.connectFd`.
  static Future<int> establish({
    required String ipAddr,
    required String netmask,
    int mtu = 1400,
  }) async {
    final fd = await _method.invokeMethod<int>('establish', {
      'ipAddr': ipAddr,
      'netmask': netmask,
      'mtu': mtu,
    });
    if (fd == null) {
      throw StateError('establish returned null fd');
    }
    return fd;
  }

  /// Tear down the tunnel and close the fd.
  static Future<void> stop() => _method.invokeMethod<void>('stop');
}
