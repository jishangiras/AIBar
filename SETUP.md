# AIBar – Getting Started

## Prerequisites

| Tool | Version | Install |
|------|---------|---------|
| Rust | 1.78+ | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| Node | 20+ | `brew install node` |
| Tauri CLI | 2.x | bundled via `npm run tauri` |

## 1. Install dependencies

```sh
npm install
```

## 2. Generate app icons

Create a 1024×1024 PNG source icon, then:

```sh
npm run tauri icon path/to/your-icon.png
```

This generates all required icon sizes into `src-tauri/icons/`.

**Tray icon:** place a 32×32 (or 64×64 @2x) PNG at `src-tauri/icons/tray-icon.png`.
On macOS, set it to a monochrome template image for auto dark/light adaptation.

## 3. Development

```sh
npm run tauri dev
```

The app starts minimized to tray. Left-click the tray icon to open the popup.
Right-click for the native menu (Refresh All, Settings, Quit).

## 4. Add API keys

Right-click tray → Settings, then paste your API keys.
Keys are stored in the OS keychain (macOS Keychain / Windows Credential Manager).

## 5. Build for production

```sh
npm run tauri build
```

Output: `src-tauri/target/release/bundle/`

## Adding a new AI service

1. Create `src-tauri/src/services/myservice.rs` implementing the `AiService` trait.
2. Declare `pub mod myservice;` in `src-tauri/src/services/mod.rs`.
3. Add it to the `services` vec in `lib.rs`, `polling.rs`, and the `match` arms in `commands.rs`.
4. Add its icon/meta to the frontend maps in `ServiceCard.svelte` and `settings/+page.svelte`.
