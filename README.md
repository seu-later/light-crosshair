# LightCrosshair

LightCrosshair is a small native Windows 10/11 x64 utility that displays a transparent PNG at the physical-pixel center of a selected monitor. It is a normal desktop overlay: it does not inject code, hook graphics APIs, inspect games, access the network, or interact with anti-cheat software.

The architecture follows the low-work philosophy of [Simple Crosshair Overlay](https://github.com/zkxs/simple-crosshair-overlay): a crosshair-sized window, cached image data, and redraws only after a real state change. It uses native `RegisterHotKey` messages instead of polling. The native settings controls provide the focused customization ideas from [CrossOver](https://github.com/lacymorrow/crossover) without its Electron runtime, updater, animations, or broader feature set.

## Run

Download or copy `LightCrosshair.exe` anywhere and run it. No installer or runtime is required. On the first run, the settings window opens so you can load a transparent PNG. After that, the app lives in the system tray; closing Settings hides that window but leaves the overlay running.

Configuration is saved only when a setting changes, at:

```text
%APPDATA%\LightCrosshair\config.json
```

Default global shortcuts:

| Action | Shortcut |
| --- | --- |
| Center | `Ctrl + Alt + C` |
| Hide bind | `Ctrl + Alt + H` (disabled until a mode is selected) |
| Show bind | `Ctrl + Alt + S` (disabled until a mode is selected) |

Click a shortcut field and press a new combination to rebind it; with the field focused, clicking it again captures mouse buttons too (left, right, middle, and both side buttons). Windows reports a clear error if another application already owns the shortcut.

The bottom row holds Center + Reset (zero X/Y offsets and recenter), a Hide/Show toggle button whose label follows the current visibility state, and Exit. Closing Settings hides to the tray. The tray menu mirrors the same actions and greys out the inapplicable Show/Hide entry.

## Hide / Show

The hide/show binds are disabled by default. Pick a mode in the settings window:

- **Off** — the binds do nothing.
- **Toggle** — separate Hide and Show shortcuts: press the Hide shortcut or mouse button to hide the crosshair, press the Show one to display it again.
- **Hold** — a single shortcut: hold it down to hide the crosshair; release it to show again.

Only the bind fields used by the selected mode are shown.

Keyboard binds use native hotkey registration where possible; mouse buttons and hold mode use low-level input hooks that observe events without swallowing them, so games and applications still receive every click and keystroke.

## Image processing

- PNG is decoded once and retained in memory.
- Tint multiplies the source RGB channels by the selected color while preserving alpha. A white pixel therefore becomes the exact tint color; black remains black.
- Exact mode replaces every opaque pixel with the tint color while keeping the source alpha mask, so non-white PNGs recolor cleanly (red source + green tint yields green, not black). Leave Exact off for multiply/filter behavior.
- The image label shows filename with dimensions (e.g. `cross.png (32x32)`), Clear unloads the image and frees its buffers, and an invalid hex entry reverts to the last valid color on focus loss instead of failing silently.
- Scale ranges from `0.00x` through `5.00x`. Use the slider or type an exact multiplier in the scale box; `0.00x` hides the overlay without unloading the image.
- Opacity ranges from `0%` through `100%` with its own slider and text box.
- X/Y offset sliders shift the crosshair up to 100 pixels away from the monitor center in either direction. Centering resets both offsets to zero.
- Scaling uses nearest-neighbor sampling and final dimensions are rounded to whole physical pixels.
- Moving or recentering the overlay does not decode or reprocess the image.

## Build

Install the stable x64 MSVC Rust toolchain, then run:

```powershell
cargo build --release
```

The portable executable is produced at:

```text
target\release\LightCrosshair.exe
```

The executable embeds `assets\icon.ico` (rebuilt from `assets\icon.png`) as its Windows icon, so the file, taskbar, and tray all show the app icon.

Development checks:

```powershell
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

Release builds use fat LTO, one codegen unit, stripped symbols, and abort-on-panic.

## Xbox Game Bar build

`gamebar\LightCrosshairGameBar` is a separate Xbox Game Bar widget build. Game Bar hosts its pinned widget in its own overlay space, so it is the supported alternative for games where a normal desktop overlay is not visible.

It retains PNG loading, tint, scale, opacity, X/Y offset, visibility, and editable center/hide/show shortcuts. Game Bar controls the widget’s monitor and final position: use the Center shortcut (`Ctrl+Alt+C` by default) and then pin it from the widget title bar. The Game Bar API does not permit an app to choose a monitor or force the pin state.

Build the debug package with the Visual Studio UWP build tools:

```powershell
msbuild gamebar\LightCrosshairGameBar\LightCrosshairGameBar.csproj /restore /p:Configuration=Debug /p:Platform=x64
```

The package is an MSIX/AppX-style UWP artifact rather than a portable executable. For a local Debug build, right-click `gamebar\LightCrosshairGameBar\Install-GameBar.ps1` and choose **Run with PowerShell**; do not open the `.msix` or the generated `Install.ps1` by itself. The project installer trusts the development certificate and supplies the matching x64 framework dependencies without invoking the obsolete developer-license flow. Approve its administrator prompt only for a package you built yourself. A distributed build should use a Microsoft Store signing identity. Game Bar availability remains game- and system-dependent.

## Windows limitations

Desktop overlays work over normal applications, windowed games, and borderless-windowed games. Exclusive fullscreen can bypass the Desktop Window Manager and prevent the crosshair from appearing; use borderless fullscreen instead. Some games or anti-cheat policies may prohibit overlays regardless of implementation, so follow the rules for the game you play.

LightCrosshair is unsigned, so Windows SmartScreen may show an unrecognized-app warning for locally built or downloaded copies.
