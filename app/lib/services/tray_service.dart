import 'dart:io' show Platform;

import 'package:tray_manager/tray_manager.dart';
import 'package:window_manager/window_manager.dart';

/// Wires up "minimize to menu-bar tray on window close" behavior for desktop.
///
/// Closing the window hides it (the app — and any active VPN tunnel — keeps
/// running) instead of quitting. The tray icon reopens the window, and only
/// "Quit" actually terminates the app.
class TrayService with TrayListener, WindowListener {
  TrayService._();
  static final TrayService instance = TrayService._();

  bool _initialized = false;

  /// Call once, after `windowManager.ensureInitialized()`, before the first
  /// frame. No-op on platforms without a menu-bar tray.
  Future<void> init() async {
    if (_initialized) return;
    if (!(Platform.isMacOS || Platform.isWindows || Platform.isLinux)) return;
    _initialized = true;

    // Intercept the red close button so it hides instead of terminating.
    await windowManager.setPreventClose(true);
    windowManager.addListener(this);

    trayManager.addListener(this);
    // The tray image is the colored app icon, so it is NOT a template image
    // (template mode uses only alpha and would render it as a solid blob). Swap
    // in a monochrome glyph and set isTemplate:true if a native menu-bar look is
    // wanted later.
    await trayManager.setIcon('assets/tray_icon.png');
    await _rebuildMenu();
  }

  Future<void> _rebuildMenu() async {
    await trayManager.setContextMenu(
      Menu(
        items: [
          MenuItem(key: 'show', label: 'Show netplane'),
          MenuItem.separator(),
          MenuItem(key: 'quit', label: 'Quit'),
        ],
      ),
    );
  }

  Future<void> _showWindow() async {
    await windowManager.show();
    await windowManager.focus();
  }

  // --- WindowListener -------------------------------------------------------

  @override
  void onWindowClose() async {
    // preventClose is on, so instead of closing we just hide to the tray.
    if (await windowManager.isVisible()) {
      await windowManager.hide();
    }
  }

  // --- TrayListener ---------------------------------------------------------

  @override
  void onTrayIconMouseDown() {
    // Left click: show the context menu (matches typical macOS menu-bar apps).
    trayManager.popUpContextMenu();
  }

  @override
  void onTrayIconRightMouseDown() {
    trayManager.popUpContextMenu();
  }

  @override
  void onTrayMenuItemClick(MenuItem menuItem) async {
    switch (menuItem.key) {
      case 'show':
        await _showWindow();
        break;
      case 'quit':
        // Allow the real close this time, then terminate.
        await windowManager.setPreventClose(false);
        await windowManager.destroy();
        break;
    }
  }

  void dispose() {
    trayManager.removeListener(this);
    windowManager.removeListener(this);
  }
}
