<p align="center">
  <img src="assets/logo.svg" alt="AIBar" width="480"/>
</p>

<p align="center">
  A native macOS menu bar app that shows live rate-limit and health status for your AI API keys — Claude, ChatGPT, Gemini, Grok, and Perplexity, all in one glance.
</p>

<p align="center">
  <img src="https://img.shields.io/badge/platform-macOS-lightgrey?style=flat-square"/>
  <img src="https://img.shields.io/badge/tauri-2.0-blue?style=flat-square"/>
  <img src="https://img.shields.io/badge/svelte-5-orange?style=flat-square"/>
  <img src="https://img.shields.io/badge/rust-1.78+-red?style=flat-square"/>
  <img src="https://img.shields.io/badge/license-MIT-green?style=flat-square"/>
</p>

---

## What it does

AIBar sits in your macOS menu bar and polls each AI service's rate-limit headers in the background. Click the tray icon to see a live dashboard of:

- **Health status** — green / yellow / red per service
- **Rate-limit progress bars** — requests remaining vs. limit
- **Per-limit detail panel** — token limits, request limits, reset times
- **Aggregate indicator** — tray icon reflects the worst active service

API keys are stored in your **OS keychain** — never on disk or in a remote server.

---

## Services

| Service | Provider | Notes |
|---------|----------|-------|
| Claude | Anthropic | Reads `anthropic-ratelimit-*` response headers |
| ChatGPT | OpenAI | Reads `x-ratelimit-*` response headers |
| Gemini | Google | Key validity check via free `/v1beta/models` endpoint |
| Grok | xAI | OpenAI-compatible endpoint |
| Perplexity | Perplexity AI | Chat completions endpoint |

> **How it works:** For services that expose rate-limit headers (Claude, ChatGPT, Grok, Perplexity), AIBar makes a minimal 1-token completion request every 10 minutes — costing roughly $0.000001 per check — and reads the headers off the response. No billing API access required, just your standard API key.

---

## Installation

### Prerequisites

- macOS 10.15 Catalina or later
- [Rust](https://rustup.rs/) 1.78+
- [Node.js](https://nodejs.org/) 18+
- Xcode Command Line Tools (`xcode-select --install`)

### Build from source

```sh
# Clone the repo
git clone https://github.com/jishangiras/aibar.git
cd aibar

# Install JS dependencies
npm install

# Run in development mode (opens the tray app with hot reload)
npm run tauri dev

# Build a production .app bundle
npm run tauri build
# Output: src-tauri/target/release/bundle/macos/AIBar.app
```

The built `.app` can be dragged to `/Applications` like any other Mac app.

---

## Usage

1. Launch AIBar — it appears in the menu bar (no Dock icon).
2. Click the tray icon to open the popup.
3. Click **⚙ Settings** to add your API keys. Each key is stored in the OS keychain.
4. Once connected, service cards appear with live status.
5. Click any card to see the full detail panel with per-limit breakdowns.

AIBar refreshes automatically every 10 minutes with a small random jitter. Hit **↻** in the header to force an immediate refresh.

---

## Stack

| Layer | Technology |
|-------|-----------|
| Desktop shell | [Tauri 2.0](https://tauri.app/) |
| Frontend | [Svelte 5](https://svelte.dev/) + SvelteKit 2 (adapter-static) |
| Styling | Tailwind CSS 3 — dark-first, adapts to system theme |
| Backend | Rust — `tokio`, `reqwest`, `keyring` |
| State bridge | Tauri commands + events (`invoke` / `emit`) |

---

## Project structure

```
aibar/
├── src/                    Svelte frontend
│   ├── routes/
│   │   ├── +page.svelte    Main popup (service grid + settings view)
│   │   └── settings/       (legacy route, kept for deep-link compat)
│   └── lib/
│       ├── components/     ServiceCard, StatusDot, ProgressBar, DetailPanel
│       ├── stores/         Svelte 5 rune store — listens to service-updated events
│       ├── api/tauri.ts    Typed invoke() wrappers
│       └── types.ts        Shared types mirroring Rust structs
│
└── src-tauri/              Rust backend
    └── src/
        ├── lib.rs          Tauri builder + plugin registration
        ├── state.rs        AppState (Arc<RwLock<_>> fields)
        ├── commands.rs     Tauri commands exposed to the frontend
        ├── polling.rs      Background refresh loop with ±10% jitter
        ├── tray.rs         Menu bar icon + click handler
        └── services/       One module per AI provider
```

---

## Adding a new service

1. Create `src-tauri/src/services/myservice.rs` implementing the `AiService` trait (use `claude.rs` as a template).
2. Add `pub mod myservice;` to `src-tauri/src/services/mod.rs`.
3. Register it in `all_services()` in `lib.rs`.
4. Add a match arm in `refresh_service` / `refresh_all_services` in `commands.rs`.
5. Add the icon and display name to the `SERVICES` array in `+page.svelte`.

---

## Linux build (Docker)

```sh
docker compose -f docker-compose.build.yml up --build
# .deb and .AppImage appear in ./dist/
```

macOS and Windows builds require their native environments or GitHub Actions runners.

---

## Contributing

Pull requests are welcome. For larger changes, open an issue first to discuss the approach.

```sh
npm run check        # TypeScript + Svelte type-check
cargo check          # Rust type-check (from src-tauri/)
npm run tauri dev    # Full dev loop
```

---

## License

MIT — see [LICENSE](LICENSE).
