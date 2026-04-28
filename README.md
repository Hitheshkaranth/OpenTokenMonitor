<div align="center">

<img src="./public/open_token_monitor_icon.png" alt="OpenToken Monitor" width="120" />

# OpenToken Monitor

**The unified, local-first desktop monitor for Claude, Codex, and Gemini.**

One window for every usage gauge, cost trend, and recent prompt — without handing your keys to a SaaS dashboard.

[![Tauri](https://img.shields.io/badge/Tauri-2.x-FFC131?style=for-the-badge&logo=tauri&logoColor=white)](https://tauri.app/)
[![Rust](https://img.shields.io/badge/Rust-2021-CE422B?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![React](https://img.shields.io/badge/React-19-20232A?style=for-the-badge&logo=react&logoColor=61DAFB)](https://react.dev/)
[![TypeScript](https://img.shields.io/badge/TypeScript-5.8-3178C6?style=for-the-badge&logo=typescript&logoColor=white)](https://www.typescriptlang.org/)
[![Vite](https://img.shields.io/badge/Vite-7-646CFF?style=for-the-badge&logo=vite&logoColor=white)](https://vitejs.dev/)
[![SQLite](https://img.shields.io/badge/SQLite-bundled-003B57?style=for-the-badge&logo=sqlite&logoColor=white)](https://www.sqlite.org/)

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](./LICENSE)
[![Version](https://img.shields.io/badge/version-0.3.3-blue.svg)](https://github.com/Hitheshkaranth/OpenTokenMonitor/releases)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey.svg)](#-installation)
[![Local-First](https://img.shields.io/badge/data-local--first-success.svg)](#-data--privacy)

[Download](#-installation) ·
[Features](#-features) ·
[Screens](#-screens) ·
[Architecture](./ARCHITECTURE.md) ·
[Contributing](#-contributing)

</div>

---

## ✨ Why OpenToken Monitor?

You're juggling **three coding agents** at once — Claude, Codex, and Gemini — each with its own dashboard, its own quota window, and its own pricing page. OpenToken Monitor stitches them into a **single tray-resident desktop app** that:

- 📊 reads your **local CLI artifacts first** (`~/.claude`, `~/.codex`, `~/.gemini/tmp`)
- 🔐 augments them with **live OAuth fetches** when credentials are present — never proxied through a third party
- 💸 computes **accurate per-model cost** using current Q1 2026 published rates
- 🛟 keeps providers **visible even when offline**, surfacing health status instead of silently dropping them

Everything stays on your machine. Snapshots persist in a local SQLite store; nothing leaves the device.

---

## 🚀 Features

### 🎯 Core Dashboard

- **Unified overview** — Claude · Codex · Gemini in one glance, with live usage rings, trend sparklines, and provider health badges
- **Per-provider detail pages** with cost history, model breakdowns, alert thresholds (75 % / 90 % / 95 %), and recent-prompt activity
- **Projects view** — recent activity rolled up by workspace with cross-model spend attribution and command summaries
- **Compact widget mode** — fixed-size always-on-top panel for at-a-glance gauges and reset countdowns

### 🔌 Provider Intelligence

| Provider | Sources | Windows tracked |
|---|---|---|
| **Claude** | Anthropic OAuth usage API + `~/.claude/projects` local logs | 5-hour rolling, 7-day, Opus weekly, extra-credits |
| **Codex (OpenAI)** | Bearer / cookie / RPC fetchers + `~/.codex/sessions` | Daily, model breakdown |
| **Gemini** | Google OAuth quota API + `~/.gemini/tmp` session files + `gemini --stats` CLI | **60 req/min**, **1000 req/day** (free tier, midnight Pacific reset) |

### 🛡️ Resilience Built-In

- **Smart OAuth backoff** — separate cooldowns for success (120 s) and failure (25 s), so one transient 429 doesn't pin Claude to local-mode for two minutes
- **Stale-cache fallback** — last good snapshot stays on screen marked `stale` if a fetch fails
- **Reactive file watching** — `notify` watchers refresh the affected provider the moment a CLI session file changes
- **Single-instance enforcement** — autostart launch + manual click no longer fight over the SQLite store
- **Bundled WebView2 bootstrapper** — Windows MSI installs cleanly on machines without WebView2 pre-installed

### 💰 Accurate Cost Estimation (Q1 2026 rates)

All model rates live in a single source-of-truth: [`src-tauri/src/pricing.rs`](./src-tauri/src/pricing.rs).

| Family | Tier | Rate (input / output per 1M tokens) |
|---|---|---|
| Claude | Opus 4.x | $15.00 / $75.00 |
| Claude | Sonnet 4.x | $3.00 / $15.00 |
| Claude | Haiku 4.5 | $1.00 / $5.00 |
| OpenAI | GPT-5 | $1.25 / $10.00 |
| OpenAI | GPT-5 mini / nano | $0.25 / $2.00 · $0.05 / $0.40 |
| OpenAI | o3 / o4-mini | $2.00 / $8.00 · $1.10 / $4.40 |
| Gemini | 2.5 Pro / Flash / Flash-Lite | $1.25 / $10.00 · $0.30 / $2.50 · $0.10 / $0.40 |

Caching discounts and cache-write surcharges are applied where each provider exposes them.

### ⌨️ Keyboard Shortcuts

| Shortcut | Action |
|---|---|
| <kbd>1</kbd> / <kbd>2</kbd> / <kbd>3</kbd> | Jump to Claude / Codex / Gemini |
| <kbd>4</kbd> | Open Projects |
| <kbd>Esc</kbd> | Return to overview |
| <kbd>Ctrl</kbd>+<kbd>R</kbd> / <kbd>⌘</kbd>+<kbd>R</kbd> | Refresh all providers |
| <kbd>Ctrl</kbd>+<kbd>,</kbd> / <kbd>⌘</kbd>+<kbd>,</kbd> | Open settings |

---

## 📸 Screens

| Overview | Projects |
|---|---|
| ![Overview](./docs/images/overview-0.3.1.png) | ![Projects](./docs/images/projects-0.3.1.png) |

| Provider Detail | Widget Mode |
|---|---|
| ![Provider detail](./docs/images/provider-detail-0.3.1.png) | ![Widget](./docs/images/widget-0.3.1.png) |

| Settings |
|---|
| ![Settings](./docs/images/settings-0.3.1.png) |

---

## 📦 Installation

### Pre-built binaries

Grab the latest installer for your OS from [GitHub Releases](https://github.com/Hitheshkaranth/OpenTokenMonitor/releases/latest):

[![Windows](https://img.shields.io/badge/Windows-MSI-0078D6?style=for-the-badge&logo=windows&logoColor=white)](https://github.com/Hitheshkaranth/OpenTokenMonitor/releases/latest)
[![macOS Intel](https://img.shields.io/badge/macOS-Intel%20DMG-000000?style=for-the-badge&logo=apple&logoColor=white)](https://github.com/Hitheshkaranth/OpenTokenMonitor/releases/latest)
[![macOS Apple Silicon](https://img.shields.io/badge/macOS-Apple%20Silicon%20DMG-000000?style=for-the-badge&logo=apple&logoColor=white)](https://github.com/Hitheshkaranth/OpenTokenMonitor/releases/latest)
[![Linux](https://img.shields.io/badge/Linux-DEB-FCC624?style=for-the-badge&logo=linux&logoColor=black)](https://github.com/Hitheshkaranth/OpenTokenMonitor/releases/latest)

### Build from source

> **Prerequisites:** Node.js 18+, Rust stable, and your OS-specific [Tauri 2 dependencies](https://tauri.app/start/prerequisites/).

```bash
# Clone & install
git clone https://github.com/Hitheshkaranth/OpenTokenMonitor.git
cd OpenTokenMonitor
npm install

# Run in dev mode (hot-reload)
npm run tauri dev

# Production build
npm run tauri build              # all platforms
npm run tauri:build:win          # Windows NSIS installer
npm run tauri:build:mac          # macOS app bundle + installer
```

---

## 🧱 Tech Stack

<table>
<tr>
<td valign="top" width="50%">

### 🦀 Backend (Rust)

- [**Tauri 2**](https://tauri.app/) — desktop shell & IPC
- [**Tokio**](https://tokio.rs/) — async runtime
- [**Reqwest**](https://github.com/seanmonstar/reqwest) — HTTPS with rustls
- [**Rusqlite**](https://github.com/rusqlite/rusqlite) — bundled SQLite persistence
- [**Notify**](https://github.com/notify-rs/notify) — filesystem watchers
- [**Chrono · Serde · Async-trait**](https://crates.io)
- `tauri-plugin-single-instance`, `tauri-plugin-autostart`

</td>
<td valign="top" width="50%">

### ⚛️ Frontend (TypeScript)

- [**React 19**](https://react.dev/) — UI runtime
- [**Zustand**](https://zustand-demo.pmnd.rs/) — store layer
- [**Recharts**](https://recharts.org/) — usage trend graphs
- [**Framer Motion**](https://www.framer.com/motion/) — micro-interactions
- [**Lucide React**](https://lucide.dev/) — icon set
- [**Vite 7**](https://vitejs.dev/) — bundler
- [**Tailwind**](https://tailwindcss.com/) — utility styling

</td>
</tr>
</table>

---

## 🏗️ Architecture

```
┌──────────────────────────────────────────────────────────┐
│  React 19  ·  Zustand stores  ·  hooks (resize / kbd)    │
└──────────────────┬───────────────────────────────────────┘
                   │  Tauri invoke()  /  usage-updated event
┌──────────────────┴───────────────────────────────────────┐
│  commands.rs  →  aggregator  →  provider registry        │
│         │                          ├─ Claude  (OAuth + logs)
│         │                          ├─ Codex   (bearer/cookie/RPC + logs)
│         │                          └─ Gemini  (OAuth + CLI + logs)
│         ↓                                                │
│  UsageStore (SQLite)  ·  pricing.rs  ·  alerts.rs        │
│  tray.rs  ·  watchers (poll + filesystem)                │
└──────────────────────────────────────────────────────────┘
```

Full module map and data-flow walkthrough → [**ARCHITECTURE.md**](./ARCHITECTURE.md)

Key entry points:

- 🚪 Frontend root — [`src/App.tsx`](./src/App.tsx)
- 🔌 Backend entry — [`src-tauri/src/lib.rs`](./src-tauri/src/lib.rs)
- 📞 Tauri commands — [`src-tauri/src/commands.rs`](./src-tauri/src/commands.rs)
- 💱 Cost rate tables — [`src-tauri/src/pricing.rs`](./src-tauri/src/pricing.rs)
- 🗄️ SQLite layer — [`src-tauri/src/usage/store.rs`](./src-tauri/src/usage/store.rs)

---

## 🔒 Data & Privacy

- 🏠 **100 % local.** All snapshots, costs, and recent activity are stored in a local SQLite file under your OS app-data directory.
- 🔑 **Credentials never leave the device.** OAuth tokens are read from the official CLI keychain entries; API requests go directly from your machine to Anthropic / OpenAI / Google.
- 🚫 **No telemetry.** No analytics, no crash reporting, no phone-home.
- 🧹 **Reset by deleting** the app-data SQLite file (`%APPDATA%\com.opentokenmonitor.desktop\usage.db` on Windows; equivalents on macOS / Linux).

---

## 🗺️ Roadmap

- [ ] Per-project budget alerts with native notifications
- [ ] Export usage reports to CSV / JSON / PDF
- [ ] Custom refresh cadences per provider
- [ ] Cursor / Aider / Continue.dev provider adapters
- [ ] Multi-month spend forecasting

Have a request? [Open an issue](https://github.com/Hitheshkaranth/OpenTokenMonitor/issues/new).

---

## 🤝 Contributing

Contributions are welcome — start with the architecture map and pricing module:

1. Read [`ARCHITECTURE.md`](./ARCHITECTURE.md) for the module layout.
2. For new providers: implement the `UsageProvider` trait in `src-tauri/src/providers/<name>/` and register it in `registry.rs`.
3. For pricing updates: edit `src-tauri/src/pricing.rs` and bump the review-date stamp at the top of the file.
4. Run the test suite before opening a PR:

```bash
cd src-tauri && cargo test --lib
npx tsc --noEmit
```

---

## 📜 License

Released under the [MIT License](./LICENSE) — free for personal and commercial use.

---

<div align="center">

**Built with ❤️ for developers who use more than one AI agent.**

OpenTokenMonitor is not affiliated with Anthropic, OpenAI, or Google.

[⬆ Back to top](#opentoken-monitor)

</div>
