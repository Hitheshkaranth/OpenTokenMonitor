![OpenTokenMonitor](./docs/images/open_token_display.png)

Local-first desktop monitor for Claude, Codex, and Gemini usage.

[![Tauri](https://img.shields.io/badge/Tauri-2.x-24C8DB?logo=tauri&logoColor=white)](https://tauri.app/)
[![React](https://img.shields.io/badge/React-19-20232A?logo=react&logoColor=61DAFB)](https://react.dev/)
[![TypeScript](https://img.shields.io/badge/TypeScript-5.x-3178C6?logo=typescript&logoColor=white)](https://www.typescriptlang.org/)
[![Rust](https://img.shields.io/badge/Rust-2021-000000?logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Vite](https://img.shields.io/badge/Vite-7.x-646CFF?logo=vite&logoColor=white)](https://vite.dev/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](./LICENSE)

![OpenTokenMonitor screenshot](./docs/images/app-screenshot-2026-03-08.png)

## What It Does

OpenTokenMonitor pulls together local CLI logs, live provider surfaces, and local desktop UX into one app for tracking AI usage without pretending every provider exposes the same kind of quota.

- Tracks Claude, Codex, and Gemini in one dashboard.
- Distinguishes `exact`, `approximate`, and `percent-only` usage windows.
- Carries provenance on snapshots: `official`, `internal`, or `derived_local`.
- Surfaces per-model mix for Claude and Codex from local usage logs.
- Generates alert thresholds at `75%`, `90%`, and `95%`.
- Exposes an exportable usage report shape for Forge / ACP-style workflows.
- Runs as a native Tauri desktop app with tray support, widget mode, and demo mode.

## Current Feature Set

### Usage Windows

- Claude: rolling `5h` and `7d` windows plus local token accounting.
- Codex: session / weekly windows plus local per-model token and cost tracking.
- Gemini: daily and request-rate style windows, with local fallbacks when live quota data is not available.

### Accuracy Model

- Exact windows are shown only when the underlying provider surface behaves like a real counter.
- Approximate windows are derived from local logs and labeled accordingly.
- Percent-only windows are rendered as utilization only and do not fake token remaining.

### Model Mix And Alerts

- Provider cards show top model contributors by tokens and estimated cost.
- Diagnostics show provider health, latest snapshot source, and active alert counts.
- Threshold alerts are computed from current window utilization and included in the exported usage report.

### Local-First Design

- Rust/Tauri backend scans local provider data under `.claude`, `.codex`, and `.gemini`.
- React frontend renders provider cards, overview panels, charts, alerts, and diagnostics.
- Live refresh works through Tauri commands, polling, file watching, and stored snapshots.

## Provider Notes

| Provider | Primary Windows | Main Sources | Notes |
| --- | --- | --- | --- |
| Claude | `5h`, `7d`, optional weekly model window | local logs, OAuth usage surface | Subscriber windows may be percent-only. |
| Codex | `session`, `weekly` | local logs, CLI, internal ChatGPT surfaces | ChatGPT plan windows are kept separate from API model accounting. |
| Gemini | `daily`, request-rate/session window | CLI stats, live quota surface, local session files | No official weekly limit is assumed. |

## Stack

- Frontend: React 19, TypeScript, Zustand, Recharts, Framer Motion, Vite
- Desktop shell: Tauri 2
- Backend: Rust, Tokio, Reqwest, Rusqlite, Notify

## Quick Start

### Prerequisites

- Node.js 18+ (`20+` recommended)
- Rust stable (`rustup`)
- Tauri platform prerequisites: https://v2.tauri.app/start/prerequisites/

### Install

```bash
git clone https://github.com/side-quests/OpenTokenMonitor.git
cd OpenTokenMonitor
npm install
```

### Run

```bash
# Web UI only
npm run dev

# Desktop app (Tauri dev mode)
npm run tauri dev
```

### Build

```bash
# Frontend bundle
npm run build

# Desktop executable and installers
npm run tauri build
```

## Scripts

- `npm run dev` - start the Vite dev server
- `npm run build` - type-check and build the frontend
- `npm run preview` - preview the production frontend bundle
- `npm run tauri dev` - run the Tauri desktop app in development
- `npm run tauri build` - build the desktop executable and installers

## Build Outputs

After `npm run tauri build`, Windows artifacts are written under `src-tauri/target/release/` and `src-tauri/target/release/bundle/`.

- Direct executable: `src-tauri/target/release/open-token-monitor.exe`
- MSI installer: `src-tauri/target/release/bundle/msi/`
- NSIS installer: `src-tauri/target/release/bundle/nsis/`

## Docs

- Architecture: [ARCHITECTURE.md](./ARCHITECTURE.md)
- Provider research: [docs/research/provider-usage-research-2026-03-12.md](./docs/research/provider-usage-research-2026-03-12.md)
- Forge execution guide: [docs/forge/README.md](./docs/forge/README.md)
- Forge swarm manifest: [docs/forge/swarm-manifest.json](./docs/forge/swarm-manifest.json)

## Validation

The current implementation slice has been validated with:

- `cargo test`
- `npm.cmd run build`
- `npm.cmd run tauri build`

## Limitations

- Provider precision is intentionally uneven because the providers expose different surfaces.
- Gemini weekly usage should be treated as derived analytics unless Google publishes a first-class weekly quota surface.
- Claude and Codex subscriber windows may be percent-only even when local token accounting is exact.
- Internal provider endpoints can change; the app labels provenance so the UI can reflect that honestly.

## Project Layout

```text
.
|- src/                  # React UI, hooks, stores, types, utilities
|- src-tauri/            # Rust/Tauri backend, providers, usage storage, watchers
|- public/               # Static assets
|- docs/images/          # README screenshots
|- docs/research/        # Provider research memos
|- docs/forge/           # Forge guide, swarm manifest, prompt pack
|- ARCHITECTURE.md       # Technical architecture notes
```

## Releases

https://github.com/side-quests/OpenTokenMonitor/releases

## License

MIT. See [LICENSE](./LICENSE).
