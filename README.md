# OpenTokenMonitor

<p align="center">
  <img src="./open_token_monitor_icon.png" width="80" alt="OpenTokenMonitor logo" />
</p>

OpenTokenMonitor is a desktop Tauri app that tracks local AI CLI activity in one always-on-top widget.

It reads local history/state from:
- Claude Code (`~/.claude`)
- OpenAI Codex (`~/.codex`)
- Gemini CLI (`~/.gemini`)

Optional provider API keys can be added in Settings for usage/cost views.

![OpenTokenMonitor screenshot](./open_token_display.png)

## Features

- Live CLI activity stream for Claude, Codex, and Gemini.
- Provider summary cards (commands, sessions, projects, and cached Claude usage when available).
- Home limits view with recent usage windows.
- 7-day trends chart.
- Frameless, draggable, always-on-top mini window.
- Tray icon with show/hide and quit actions.

## Tech Stack

- Frontend: React + TypeScript + Vite
- Desktop shell: Tauri v2
- Backend: Rust (Tokio + notify filesystem watchers)

## Prerequisites

- Node.js 18+ (Node.js 20+ recommended)
- npm 9+
- Rust stable toolchain (`rustup`)
- Tauri system prerequisites for your OS: <https://v2.tauri.app/start/prerequisites/>

## Quick Start (Development)

```bash
git clone https://github.com/<your-org>/OpenTokenMonitor.git
cd OpenTokenMonitor
npm install
npm run tauri dev
```

## Usage

1. Start the app (`npm run tauri dev` in development, or launch installed app).
2. Run commands in Claude/Codex/Gemini CLIs as normal.
3. Watch `Live` and `Stats` tabs update from local history files.
4. Open `Settings` to:
   - Add optional API keys.
   - Configure refresh interval.
   - Configure usage auth bridge values if needed.
5. Use `-` button to hide to tray; click tray icon or tray menu to restore.

## Build Options

### Frontend-only build

```bash
npm run build
```

Outputs static assets into `dist/`.

### Desktop app build (default targets for current OS)

```bash
npm run tauri build
```

Bundle outputs are under:
- `src-tauri/target/release/bundle/`

### Target-specific desktop build

Use Tauri arguments after `--`:

```bash
# example: Windows NSIS bundle
npm run tauri build -- --bundles nsis

# example: Linux deb bundle
npm run tauri build -- --bundles deb
```

Common bundle types by platform:
- Windows: `nsis`, `msi`
- macOS: `dmg`, `app`
- Linux: `deb`, `appimage`, `rpm`

## Quality Checks

Run these before pushing:

```bash
# TypeScript + Vite production build
npm run build

# Rust compile check for backend
cargo check --manifest-path src-tauri/Cargo.toml
```

Optional full packaging check:

```bash
npm run tauri build
```

## Project Layout

```text
.
|-- src/                 # React frontend
|-- src-tauri/           # Rust + Tauri backend
|-- public/              # Static assets
|-- docs/                # Planning notes
|-- open_token_display.png
`-- open_token_monitor_icon.png
```

## Notes

- No cloud sync is required for local CLI activity tracking.
- If a provider shows no activity, verify that provider CLI has created its local history/state files.
