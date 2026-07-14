import NetworkExtension
import Darwin
import os.log

/// macOS Packet Tunnel Provider for netplane.
///
/// The system creates the `utun` interface for this (entitled) extension process
/// and we hand its file descriptor to the `netplane_client` Rust core via the C
/// FFI declared in `PacketTunnel-Bridging-Header.h`. This is the only supported
/// way to run a TUN on macOS — a sandboxed GUI app cannot open `utun` itself.
///
/// Flow (see `startTunnel`): create transport → handshake (gets IP/netmask) →
/// `setTunnelNetworkSettings` → locate the utun fd → `netplane_client_run_fd`.
class PacketTunnelProvider: NEPacketTunnelProvider {
  private let log = OSLog(subsystem: "com.netplane.app.PacketTunnel", category: "tunnel")

  override func startTunnel(
    options: [String: NSObject]?,
    completionHandler: @escaping (Error?) -> Void
  ) {
    netplane_client_init_logger(0)
    os_log("startTunnel called", log: self.log, type: .default)

    guard
      let proto = self.protocolConfiguration as? NETunnelProviderProtocol,
      let conf = proto.providerConfiguration,
      let host = conf["host"] as? String,
      let transportType = conf["transport"] as? String,
      let authkeyPath = conf["authkey_path"] as? String,
      let publicKeyPath = conf["public_key_path"] as? String,
      let privateKeyPath = conf["private_key_path"] as? String
    else {
      completionHandler(providerError("missing provider configuration"))
      return
    }

    // 0 => client default (5000), mirroring the Rust client.
    let port = (conf["port"] as? NSNumber)?.uint16Value ?? 0
    let effectivePort: UInt16 = port == 0 ? 5000 : port
    let loopbackRelay = (conf["loopback_relay"] as? NSNumber)?.boolValue ?? false
    let noEncryption = (conf["no_encryption"] as? NSNumber)?.boolValue ?? false

    // Network + handshake work is blocking; keep it off the callback thread.
    DispatchQueue.global(qos: .userInitiated).async { [weak self] in
      guard let self = self else { return }

      os_log("creating transport %{public}@ to %{public}@:%d",
             log: self.log, type: .default, transportType, host, Int(effectivePort))
      let transport = host.withCString { h in
        transportType.withCString { t in
          netplane_create_transport(h, effectivePort, t)
        }
      }
      guard let transport = transport else {
        completionHandler(self.providerError("failed to create transport to \(host):\(effectivePort)"))
        return
      }
      os_log("transport created; starting handshake", log: self.log, type: .default)

      var result = NetplaneHandshakeResult()
      let hs = netplane_client_handshake(authkeyPath, publicKeyPath, privateKeyPath, transport, &result)
      guard hs == 0 else {
        netplane_free_transport(transport)
        completionHandler(self.providerError("handshake failed (code \(hs))"))
        return
      }

      let ipAddr = String(cString: result.ip_addr)
      let netmask = String(cString: result.netmask)
      os_log("handshake ok: ip=%{public}@ mask=%{public}@", log: self.log, type: .info, ipAddr, netmask)

      // tunnelRemoteAddress must be an IP literal; a DNS hostname fails validation
      // with "Invalid NETunnelNetworkSettings tunnelRemoteAddress". Resolve it (a
      // host that is already an IP round-trips unchanged).
      let remoteAddress = Self.resolveIPv4(host) ?? host
      let settings = NEPacketTunnelNetworkSettings(tunnelRemoteAddress: remoteAddress)
      let ipv4 = NEIPv4Settings(addresses: [ipAddr], subnetMasks: [netmask])
      // Route only the overlay subnet through the tunnel (mesh overlay, not a
      // full-traffic VPN). Falls back to the host route if the subnet can't be
      // derived.
      if let network = Self.networkAddress(ip: ipAddr, mask: netmask) {
        ipv4.includedRoutes = [NEIPv4Route(destinationAddress: network, subnetMask: netmask)]
      } else {
        ipv4.includedRoutes = [NEIPv4Route(destinationAddress: ipAddr, subnetMask: "255.255.255.255")]
      }
      settings.ipv4Settings = ipv4
      settings.mtu = 1400  // matches tundev.rs

      self.setTunnelNetworkSettings(settings) { error in
        if let error = error {
          os_log("setTunnelNetworkSettings failed: %{public}@",
                 log: self.log, type: .error, error.localizedDescription)
          netplane_client_free_handshake(&result)
          netplane_free_transport(transport)
          completionHandler(error)
          return
        }

        guard let fd = self.tunnelFileDescriptor else {
          netplane_client_free_handshake(&result)
          netplane_free_transport(transport)
          completionHandler(self.providerError("could not locate utun file descriptor"))
          return
        }

        // `run_fd` reads the handshake strings synchronously, consumes the
        // transport (do not free it here), then spawns the packet loop on the
        // Rust runtime and returns immediately.
        let rc = publicKeyPath.withCString { pub in
          privateKeyPath.withCString { priv in
            netplane_client_run_fd(fd, transport, &result, loopbackRelay, noEncryption, pub, priv)
          }
        }
        netplane_client_free_handshake(&result)

        if rc != 0 {
          completionHandler(self.providerError("failed to start tunnel loop (code \(rc))"))
        } else {
          os_log("tunnel started on fd %d", log: self.log, type: .info, fd)
          completionHandler(nil)
        }
      }
    }
  }

  override func stopTunnel(with reason: NEProviderStopReason, completionHandler: @escaping () -> Void) {
    os_log("stopTunnel: %d", log: log, type: .info, reason.rawValue)
    netplane_client_stop()
    completionHandler()
  }

  // MARK: - Helpers

  private func providerError(_ message: String) -> NSError {
    os_log("error: %{public}@", log: log, type: .error, message)
    return NSError(
      domain: "com.netplane.app.PacketTunnel",
      code: 1,
      userInfo: [NSLocalizedDescriptionKey: message]
    )
  }

  /// Resolve `host` to its first IPv4 address (dotted string). Returns nil on
  /// failure. A `host` that is already a dotted IPv4 literal round-trips
  /// unchanged (getaddrinfo with AI_NUMERICHOST-compatible input).
  private static func resolveIPv4(_ host: String) -> String? {
    var hints = addrinfo(
      ai_flags: 0, ai_family: AF_INET, ai_socktype: SOCK_STREAM,
      ai_protocol: 0, ai_addrlen: 0, ai_canonname: nil, ai_addr: nil, ai_next: nil)
    var res: UnsafeMutablePointer<addrinfo>?
    guard getaddrinfo(host, nil, &hints, &res) == 0, let info = res else { return nil }
    defer { freeaddrinfo(res) }
    var addr = info.pointee.ai_addr.withMemoryRebound(to: sockaddr_in.self, capacity: 1) {
      $0.pointee.sin_addr
    }
    var buf = [CChar](repeating: 0, count: Int(INET_ADDRSTRLEN))
    guard inet_ntop(AF_INET, &addr, &buf, socklen_t(INET_ADDRSTRLEN)) != nil else { return nil }
    return String(cString: buf)
  }

  /// Bitwise-AND an IPv4 dotted address with a dotted netmask to get the network
  /// address (e.g. 10.0.0.7 / 255.255.255.0 => 10.0.0.0).
  private static func networkAddress(ip: String, mask: String) -> String? {
    let ipParts = ip.split(separator: ".").compactMap { UInt8($0) }
    let maskParts = mask.split(separator: ".").compactMap { UInt8($0) }
    guard ipParts.count == 4, maskParts.count == 4 else { return nil }
    let net = (0..<4).map { String(ipParts[$0] & maskParts[$0]) }
    return net.joined(separator: ".")
  }

  /// Locate the `utun` file descriptor the system opened for this provider.
  ///
  /// NEPacketTunnelFlow exposes no raw fd, but the Rust core is fd-based, so we
  /// scan open descriptors for the one whose kernel-control interface name has
  /// the `utun` prefix (the technique WireGuard's macOS/iOS clients use).
  /// `SYSPROTO_CONTROL` (2) and `UTUN_OPT_IFNAME` (2) are hardcoded because the
  /// corresponding C macros aren't importable into Swift.
  private var tunnelFileDescriptor: Int32? {
    var buf = [CChar](repeating: 0, count: Int(IFNAMSIZ))
    for fd: Int32 in 0...1024 {
      var len = socklen_t(buf.count)
      if getsockopt(fd, 2, 2, &buf, &len) == 0, String(cString: buf).hasPrefix("utun") {
        return fd
      }
    }
    return nil
  }
}
