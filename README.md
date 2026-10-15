<p align="center">
  <img src="docs/icon.png" width="96" alt="ClipNext logo">
</p>

<h1 align="center">ClipNext</h1>

<p align="center">
  A small, fast and private clipboard history manager for Windows.<br>
  <a href="README.tr.md">Türkçe</a> · English
</p>

<p align="center">
  <img src="docs/screenshot-dark.png" width="320" alt="ClipNext in dark mode">
  &nbsp;
  <img src="docs/screenshot-light.png" width="320" alt="ClipNext in light mode">
</p>

ClipNext remembers everything you copy (text, links and images), keeps it on
your computer only, and lets you paste any of it again with one shortcut. It
runs silently in the system tray and uses very little memory.

## Features

- **History for text, links and images**, newest first (500 items by default, configurable)
- **Instant search**, case-insensitive and Turkish-character aware
- **Global shortcut** (default `Ctrl+Shift+V`, configurable) opens the panel right above the text field you are typing in
- **One click to paste**: pick an item and it is pasted into the app you were using
- **Pin** items so they are never deleted and stay on top
- **Light, dark or Windows default** theme, English and Turkish interface
- **Tray app**: the close button hides the window, "Quit" in the tray menu exits
- **Start with Windows** (optional, off by default)
- **100 % local**: no network access, no telemetry, no account

## Download and install

1. Open the [Releases](../../releases/latest) page and download `ClipNext.exe`.
2. Put it anywhere you like and double-click it. There is no installer.
3. Windows SmartScreen may warn about an unknown publisher because the file is
   not code-signed. Choose **More info → Run anyway**.

Requirements: Windows 10 or 11 (64-bit). The Microsoft Edge WebView2 runtime is
required; it is already part of Windows 11 and most Windows 10 installs.

## How it works

1. Start `ClipNext.exe`. A tray icon appears and the app keeps watching the clipboard in the background.
2. Copy something anywhere, as usual.
3. Press `Ctrl+Shift+V` while typing in any app. The ClipNext panel opens above the caret.
4. Pick an item with the mouse, or use the arrow keys and `Enter`. The panel closes and the item is pasted into the app you came from.

<p align="center">
  <img src="docs/screenshot-settings.png" width="320" alt="ClipNext settings">
</p>

### Keyboard

| Key | Action |
| --- | --- |
| `Ctrl+Shift+V` (configurable) | Show or hide the panel |
| `↑` / `↓` | Move the selection |
| `Enter` | Paste the selected item |
| `Esc` | Clear the search, then close the panel |
| `Ctrl+P` | Pin or unpin the selected item |
| `Delete` | Delete the selected item (search box empty) |
| `Ctrl+,` | Open or close settings |

### Settings

| Setting | Description |
| --- | --- |
| Language | System, Türkçe or English |
| Appearance | Windows default, Light or Dark |
| Keyboard shortcut | Click the button and press the new combination (at least one modifier key) |
| Paste automatically | Paste into the previous app, or only copy to the clipboard |
| Start with Windows | Launch hidden in the tray when you sign in |
| History size | How many unpinned items are kept |
| Save images / Image storage limit | Turn image capture off or cap the disk space used |

## Privacy

- All data lives in `%APPDATA%\com.clipnext.app\` (a SQLite database and an `images` folder). Nothing is sent anywhere and the app has no network permissions.
- Content that password managers mark as private (the Windows clipboard formats `ExcludeClipboardContentFromMonitorProcessing`, `Clipboard Viewer Ignore` and `CanIncludeInClipboardHistory = 0`) is never recorded.
- Image storage is bounded: files above 10 MB are skipped, large images are downscaled to 3840 px, and total image storage is capped (200 MB by default). Orphaned files are removed on start.
- You can clear the history, all images, or everything including pinned items from inside the app.

## Build from source

Requirements: Node.js 20+, Rust (stable, MSVC toolchain), Visual Studio Build
Tools with the "Desktop development with C++" workload, WebView2.

```bash
npm install
npm run tauri dev                    # run with hot reload
cd src-tauri && cargo test           # backend unit tests
npm run tauri build -- --no-bundle   # single exe: src-tauri/target/release/clipnext.exe
```

Set the `CLIPNEXT_DATA_DIR` environment variable to run with a separate data
folder (useful for testing without touching your real history).

## Project layout

```
src/              React + TypeScript + Tailwind UI
  components/     list, item, settings, dialogs, icons
  services/       backend calls, formatting, shortcut helpers
  i18n/           English and Turkish strings
src-tauri/        Rust backend (Tauri 2)
  src/db/         SQLite access, versioned migrations, tests
  src/clipboard/  clipboard listener, classification, image storage
  src/focus.rs    caret lookup, window placement, paste
```

Database schema changes are versioned migrations in
`src-tauri/src/db/migrations.rs` (tracked with `PRAGMA user_version`), so
existing history survives upgrades.

## Contributing

Issues and pull requests are welcome. Please run `cargo test` and
`npx tsc --noEmit` before opening a pull request.

## License

[MIT](LICENSE)
