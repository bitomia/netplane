# Vendored fork of `tray_manager` 0.5.3

Upstream: https://pub.dev/packages/tray_manager (BSD-3-Clause, see LICENSE)

## Why this exists

Upstream's `linux/CMakeLists.txt` raises a hard `FATAL_ERROR` unless
`ayatana-appindicator3-0.1` or `appindicator3-0.1` is installed, so *any*
Linux build of the app fails on a machine without `libayatana-appindicator3-dev`.
Flutter has no way to exclude a dependency per platform from the app's own
pubspec, and `linux/flutter/generated_plugins.cmake` is regenerated on every
build, so it cannot be patched from our side either.

The flutter tool decides which plugins a platform builds purely from the
plugin's own `flutter.plugin.platforms` map. Dropping the `linux:` entry there
makes Linux skip the plugin entirely — no CMake, no entry in the generated
plugin registrant — while macOS and Windows are untouched.

## The entire diff vs. upstream 0.5.3

`pubspec.yaml`, under `flutter.plugin.platforms`:

```diff
-      linux:
-        pluginClass: TrayManagerPlugin
```

Nothing else is changed. `linux/` is left in place (unused) to keep the diff
minimal and re-appliable.

## Consequence

There is no tray icon on Linux. `lib/services/tray_service.dart` must therefore
not run on Linux — in particular it must not call
`windowManager.setPreventClose(true)` there, or closing the window would hide it
with no tray icon to restore it and no way to quit.

## Upgrading

Re-copy the new upstream version over this directory (minus `example/`) and
re-apply the two-line deletion above.
