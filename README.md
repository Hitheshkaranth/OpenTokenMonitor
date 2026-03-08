# OpenTokenMonitor

A local-first desktop monitor for Claude, Codex, and Gemini CLI usage.

[![Tauri](https://img.shields.io/badge/Tauri-2.x-24C8DB?logo=tauri&logoColor=white)](https://tauri.app/)
[![React](https://img.shields.io/badge/React-19-20232A?logo=react&logoColor=61DAFB)](https://react.dev/)
[![TypeScript](https://img.shields.io/badge/TypeScript-5.x-3178C6?logo=typescript&logoColor=white)](https://www.typescriptlang.org/)
[![Rust](https://img.shields.io/badge/Rust-2021-000000?logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Vite](https://img.shields.io/badge/Vite-7.x-646CFF?logo=vite&logoColor=white)](https://vite.dev/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](./LICENSE)

![OpenTokenMonitor screenshot](./docs/images/app-screenshot-2026-03-08.png)

## Features

- Unified dashboard for Claude, Codex, and Gemini activity
- Local log scanning with optional live API enrichment
- Provider status cards, usage windows, trends, and cost tracking
- System tray support for always-available monitoring
- Cross-platform desktop app via Tauri (Windows, macOS, Linux)

## Supported Providers

- Claude CLI (Anthropic)
- Codex CLI (OpenAI)
- Gemini CLI (Google)

## Tech Stack

- Frontend: React 19, TypeScript, Vite, Zustand, Recharts, Framer Motion
- Desktop shell: Tauri 2
- Backend/core: Rust, Tokio, Reqwest, Rusqlite, Notify

## Quick Start

### Prerequisites

- Node.js 18+ (Node.js 20+ recommended)
- Rust stable toolchain (`rustup`)
- Tauri OS prerequisites: https://v2.tauri.app/start/prerequisites/

### Install

```bash
git clone https://github.com/side-quests/OpenTokenMonitor.git
cd OpenTokenMonitor
npm install
```

### Run (Web UI Only)

```bash
npm run dev
```

### Run (Desktop App with Tauri)

```bash
npm run tauri dev
```

### Build Frontend

```bash
npm run build
```

### Build Desktop App

```bash
npm run tauri build
```

## NPM Scripts

- `npm run dev` - Start Vite dev server
- `npm run build` - Type-check and build frontend (`tsc && vite build`)
- `npm run preview` - Preview production frontend build
- `npm run tauri dev` - Run Tauri desktop app in development
- `npm run tauri build` - Build desktop installers/bundles

## How It Works

1. Rust backend watches local CLI/session/log files and parses usage signals.
2. Frontend requests provider snapshots through Tauri commands.
3. Optional provider API calls enrich local data with live limit/usage status.
4. Aggregated state is rendered as cards, meters, trends, and overview widgets.

See [ARCHITECTURE.md](./ARCHITECTURE.md) for deeper implementation details.

## Project Structure

```text
.
|- src/                 # React UI (components, stores, hooks, styles)
|- src-tauri/           # Rust/Tauri backend and desktop config
|- public/              # Static assets
|- docs/images/         # README and documentation images
|- ARCHITECTURE.md      # System architecture notes
```

## Releases

Prebuilt binaries are published on GitHub Releases:
https://github.com/side-quests/OpenTokenMonitor/releases

## License

MIT - see [LICENSE](./LICENSE).
