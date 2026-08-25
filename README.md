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

## Windows limitations

Desktop overlays work over normal applications, windowed games, and borderless-windowed games. Exclusive fullscreen can bypass the Desktop Window Manager and prevent the crosshair from appearing; use borderless fullscreen instead. Some games or anti-cheat policies may prohibit overlays regardless of implementation, so follow the rules for the game you play.

LightCrosshair is unsigned, so Windows SmartScreen may show an unrecognized-app warning for locally built or downloaded copies.
