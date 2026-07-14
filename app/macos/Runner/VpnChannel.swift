import Cocoa
import FlutterMacOS
import NetworkExtension

/// Bridges Flutter to the macOS NetworkExtension VPN stack.
///
/// The Flutter app can't create a TUN itself (sandbox), so connect/disconnect
/// are driven here via `NETunnelProviderManager`, which installs a VPN profile
/// and launches the `PacketTunnel` extension. Status changes are streamed back
/// over an EventChannel.
///
/// Method channel: "netplane/vpn"
///   sharedContainerPath() -> String            (App Group container for keys)
///   start(config: [String: Any]) -> Void
///   stop() -> Void
///   status() -> String
/// Event channel: "netplane/vpn/status" -> status strings
class VpnChannel: NSObject {
  static let appGroup = "group.com.netplane.app"
  static let providerBundleId = "com.netplane.app.PacketTunnel"

  private var manager: NETunnelProviderManager?
  private var eventSink: FlutterEventSink?
  private var statusObserver: NSObjectProtocol?

  static func register(with messenger: FlutterBinaryMessenger) {
    let instance = VpnChannel()
    let methodChannel = FlutterMethodChannel(name: "netplane/vpn", binaryMessenger: messenger)
    methodChannel.setMethodCallHandler(instance.handle)
    let eventChannel = FlutterEventChannel(name: "netplane/vpn/status", binaryMessenger: messenger)
    eventChannel.setStreamHandler(instance)
  }

  private func handle(_ call: FlutterMethodCall, result: @escaping FlutterResult) {
    switch call.method {
    case "sharedContainerPath":
      guard let url = FileManager.default.containerURL(
        forSecurityApplicationGroupIdentifier: Self.appGroup)
      else {
        result(FlutterError(code: "no_container",
                            message: "App Group container unavailable (\(Self.appGroup))",
                            details: nil))
        return
      }
      result(url.path)

    case "start":
      guard let args = call.arguments as? [String: Any] else {
        result(FlutterError(code: "bad_args", message: "expected config map", details: nil))
        return
      }
      start(config: args, result: result)

    case "stop":
      stop(result: result)

    case "status":
      result(statusString(manager?.connection.status ?? .invalid))

    default:
      result(FlutterMethodNotImplemented)
    }
  }

  // MARK: - Tunnel lifecycle

  private func start(config: [String: Any], result: @escaping FlutterResult) {
    NETunnelProviderManager.loadAllFromPreferences { [weak self] managers, error in
      guard let self = self else { return }
      if let error = error {
        result(FlutterError(code: "load_failed", message: error.localizedDescription, details: nil))
        return
      }

      let manager = managers?.first ?? NETunnelProviderManager()
      let proto = NETunnelProviderProtocol()
      proto.providerBundleIdentifier = Self.providerBundleId
      proto.serverAddress = (config["host"] as? String) ?? "netplane"
      proto.providerConfiguration = config

      manager.protocolConfiguration = proto
      manager.localizedDescription = "netplane"
      manager.isEnabled = true

      // A save must be followed by a reload before startVPNTunnel, otherwise the
      // freshly-saved configuration isn't yet visible to the connection.
      manager.saveToPreferences { error in
        if let error = error {
          result(FlutterError(code: "save_failed", message: error.localizedDescription, details: nil))
          return
        }
        manager.loadFromPreferences { error in
          if let error = error {
            result(FlutterError(code: "reload_failed", message: error.localizedDescription, details: nil))
            return
          }
          self.manager = manager
          self.observeStatus(of: manager)
          do {
            try manager.connection.startVPNTunnel()
            result(nil)
          } catch {
            result(FlutterError(code: "start_failed", message: error.localizedDescription, details: nil))
          }
        }
      }
    }
  }

  private func stop(result: @escaping FlutterResult) {
    guard let manager = manager else {
      result(nil)
      return
    }
    manager.connection.stopVPNTunnel()
    result(nil)
  }

  // MARK: - Status streaming

  private func observeStatus(of manager: NETunnelProviderManager) {
    if let observer = statusObserver {
      NotificationCenter.default.removeObserver(observer)
    }
    statusObserver = NotificationCenter.default.addObserver(
      forName: .NEVPNStatusDidChange,
      object: manager.connection,
      queue: .main
    ) { [weak self] _ in
      guard let self = self else { return }
      self.eventSink?(self.statusString(manager.connection.status))
    }
  }

  private func statusString(_ status: NEVPNStatus) -> String {
    switch status {
    case .invalid: return "invalid"
    case .disconnected: return "disconnected"
    case .connecting: return "connecting"
    case .connected: return "connected"
    case .reasserting: return "reasserting"
    case .disconnecting: return "disconnecting"
    @unknown default: return "unknown"
    }
  }
}

extension VpnChannel: FlutterStreamHandler {
  func onListen(withArguments arguments: Any?, eventSink events: @escaping FlutterEventSink) -> FlutterError? {
    eventSink = events
    if let manager = manager {
      events(statusString(manager.connection.status))
    }
    return nil
  }

  func onCancel(withArguments arguments: Any?) -> FlutterError? {
    eventSink = nil
    return nil
  }
}
