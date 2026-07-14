# macOS Packet Tunnel — Xcode setup checklist

All source files exist in this repo. These steps wire them into the Xcode
project (adding a target + build phases by hand-editing `project.pbxproj` is
fragile, so do them once in the Xcode GUI). Open `macos/Runner.xcworkspace`.

Bundle IDs used everywhere: app `com.netplane.app`, extension
`com.netplane.app.PacketTunnel`, App Group `group.com.netplane.app`.

> ⚠️ **Signing gate.** The `com.apple.developer.networking.networkextension`
> entitlement cannot be ad-hoc (`-`) signed. Steps 2 (capabilities) and the
> final run/verify require a **paid Apple Developer account**: set
> `DEVELOPMENT_TEAM`, and register the two App IDs + the App Group + the Network
> Extensions capability in the developer portal (Xcode's automatic signing does
> this for you once a team is selected). Everything else can be set up now.

## 1. Create the target
- **File ▸ New ▸ Target… ▸ macOS ▸ Network Extension**, Provider type
  **Packet Tunnel**, Product Name `PacketTunnel`, Language Swift. Finish.
- This creates a `PacketTunnel` target, embeds `PacketTunnel.appex` into Runner
  (an *Embed App Extensions* phase on Runner), and generates stub files.
- **Delete** the generated `PacketTunnelProvider.swift` / `Info.plist` /
  `*.entitlements` (Move to Trash), then **Add Files to "Runner"…** and add the
  committed `macos/PacketTunnel/` files, with *target membership = PacketTunnel*:
  `PacketTunnelProvider.swift`, `PacketTunnel-Bridging-Header.h`, `Info.plist`,
  `PacketTunnel.entitlements`.

## 2. Signing & Capabilities (needs a team)
On **both** the `Runner` and `PacketTunnel` targets:
- Select your Team; keep automatic signing.
- **+ Capability ▸ App Groups** → add `group.com.netplane.app`.
- **+ Capability ▸ Network Extensions** → check **Packet Tunnel**.
Confirm each target's `CODE_SIGN_ENTITLEMENTS` points at its committed
`.entitlements` file (Runner already does; set PacketTunnel's to
`PacketTunnel/PacketTunnel.entitlements`).

## 3. PacketTunnel build settings
- **Swift Compiler – General ▸ Objective-C Bridging Header** =
  `PacketTunnel/PacketTunnel-Bridging-Header.h`
  (`SWIFT_OBJC_BRIDGING_HEADER`).
- **Library Search Paths** += `$(SRCROOT)/rust_artifacts`.
- **Other Linker Flags** (`OTHER_LDFLAGS`) += `-lnetplane_client`.
- **Link Binary With Libraries** → add `NetworkExtension.framework`,
  `Security.framework`, `SystemConfiguration.framework`, `libresolv.tbd`,
  `libc++.tbd` (Rust's std + tun/tokio deps pull these in).
- Set `PRODUCT_BUNDLE_IDENTIFIER = com.netplane.app.PacketTunnel`.

## 4. Cargo build phase (PacketTunnel target)
- **Build Phases ▸ + ▸ New Run Script Phase**. Drag it **above** *Compile
  Sources*. Script:
  ```sh
  "${SRCROOT}/build_rust_ext.sh"
  ```
- Uncheck *Based on dependency analysis* so it always runs.
- The script builds `netplane_client` for `$ARCHS` and writes
  `macos/rust_artifacts/libnetplane_client.a` (the search path from step 3).

## 5. Build & verify
- `flutter build macos` (or build the Runner scheme in Xcode).
- Run the app, enter server + link code, Connect → macOS shows a **"netplane
  Would Like to Add VPN Configurations"** prompt → Allow.
- In **Console.app** filter by `com.netplane.app.PacketTunnel`: expect
  `handshake ok…`, `TUN initialized from FD…`, `tunnel started on fd N`.
- `ifconfig` shows a `utun*` with the assigned overlay IP; disconnect from the
  app (Settings) stops it.

## Notes
- Keys/auth live in the App Group container (both processes share it); the Dart
  side fetches the path via the `netplane/vpn` channel (`sharedContainerPath`).
- `rust_artifacts/` is a build output — add to `.gitignore` if desired.
