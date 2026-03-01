# OpenTokenMonitor

Desktop monitor for AI CLI usage across Claude Code, OpenAI Codex, and Gemini CLI.

![OpenTokenMonitor screenshot](./open_token_display.png)

## Overview

OpenTokenMonitor is a small always-on-top desktop widget built with Tauri. It watches local CLI history/state files and gives you a unified live view of:

- recent commands
- per-provider usage summaries
- basic trend signals
- optional API-backed usage/cost views when keys are configured

The app is local-first and works without API keys for activity tracking.

## Key Features

- Real-time activity feed for Claude, Codex, and Gemini CLI events.
- Provider cards with command/session/project metrics.
- Home limits panel with recent usage windows.
- Trends tab for recent usage visualization.
- Tray integration (show/hide + quit).
- Frameless, draggable, always-on-top desktop window.

## Tech Stack

- React 19
- TypeScript 5
- Vite 7
- Tauri v2
- Rust (Tokio + notify)
- Lucide React
- Tauri plugins: store, http, fs, shell, notification

## Getting Started

### Prerequisites

- Node.js 18+ (20+ recommended)
- npm 9+
- Rust stable toolchain (`rustup`)
- OS dependencies for Tauri: <https://v2.tauri.app/start/prerequisites/>

### Development

```bash
git clone https://github.com/<your-org>/OpenTokenMonitor.git
cd OpenTokenMonitor
npm install
npm run tauri dev
```

## Usage

1. Launch the app.
2. Run `claude`, `codex`, or Gemini CLI commands in your normal workflow.
3. Track activity in `Live`, provider status in `Stats`, and trends in `Trends`.
4. Open `Settings` to configure API keys and refresh behavior.
5. Use the `-` control to hide to tray and restore from tray icon/menu.

## Build

### Frontend build

```bash
npm run build
```

### Desktop build

```bash
npm run tauri build
```

Artifacts are generated under `src-tauri/target/release/bundle/`.

### Build by bundle target

```bash
npm run tauri build -- --bundles nsis
npm run tauri build -- --bundles deb
```

Supported bundles vary by platform:

- Windows: `nsis`, `msi`
- macOS: `dmg`, `app`
- Linux: `deb`, `appimage`, `rpm`

## Quality Checks

```bash
npm run build
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
```

## Project Structure

```text
.
|-- src/         # React UI
|-- src-tauri/   # Rust + Tauri runtime
|-- public/      # Static assets
`-- docs/        # Design and implementation notes
```

## License

Licensed under the MIT License. See [LICENSE](./LICENSE).
