# OpenTokenMonitor

**A sleek, local-first desktop monitor for your AI command-line tool usage.**

<div>
  <img src="https://img.shields.io/badge/Tauri-26C0FF?style=for-the-badge&logo=tauri&logoColor=white" alt="Tauri" />
  <img src="https://img.shields.io/badge/React-20232A?style=for-the-badge&logo=react&logoColor=61DAFB" alt="React" />
  <img src="https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white" alt="Rust" />
  <img src="https://img.shields.io/badge/TypeScript-3178C6?style=for-the-badge&logo=typescript&logoColor=white" alt="TypeScript" />
  <img src="https://img.shields.io/badge/License-MIT-yellow.svg?style=for-the-badge" alt="License: MIT" />
</div>

<br/>

![OpenTokenMonitor screenshot](./open_token_display.png)

## Overview

OpenTokenMonitor is a lightweight, always-on-top desktop widget that gives you a unified, real-time view of your AI CLI tool activity. It's designed for developers who frequently use command-line AI tools and want to keep track of their usage patterns and costs without interrupting their workflow.

The application is **local-first**, meaning it works out-of-the-box by securely reading local history files. For enhanced, real-time data, you can optionally add API keys to see live token counts and rate limits directly from provider APIs.

## ✨ Key Features

-   **Unified Dashboard:** A single place to monitor usage across multiple AI providers.
-   **Live Activity Feed:** See your CLI commands from different tools appear in real-time.
-   **Provider-Specific Stats:** Get detailed metrics like commands per day, unique projects, and active sessions.
-   **Usage Trends:** Visualize your activity over time to understand your habits.
-   **System Tray Integration:** Runs discreetly in your system tray for easy access.
-   **Live API Integration (Optional):** Add your API keys to see up-to-the-minute token consumption and rate-limit data from supported providers.
-   **Cross-Platform:** Works on Windows, macOS, and Linux.

## ✅ Supported Providers

-   **Anthropic** (Claude CLI)
-   **OpenAI** (Codex CLI)
-   **Google** (Gemini CLI)

## 🚀 Installation

Pre-built binaries for Windows, macOS, and Linux are available on the **[GitHub Releases](https://github.com/side-quests/OpenTokenMonitor/releases)** page.

1.  Go to the latest release.
2.  Download the appropriate installer for your operating system (e.g., `.msi` for Windows, `.dmg` for macOS, `.deb` or `.AppImage` for Linux).
3.  Run the installer.

## ⚙️ How It Works

OpenTokenMonitor uses a hybrid approach to gather your usage data:

1.  **Local File Monitoring:** A high-performance Rust core continuously watches the history and log files created by the official Claude, Codex, and Gemini CLIs. This provides a secure, private, and key-free way to track your activity.
2.  **Direct API Calls:** For live data, the frontend uses an HTTP client to query provider APIs (e.g., Anthropic's rate-limit endpoint). This data is layered on top of the local history to give you the most accurate, real-time view possible. Your API keys are stored locally and are never sent anywhere except the intended provider.

For a more detailed technical breakdown, see the [**Architecture Document**](./ARCHITECTURE.md).

## 🧑‍💻 For Developers

Interested in contributing? Welcome!

### Prerequisites

-   **Node.js:** v18+ (v20+ recommended)
-   **Rust:** Latest stable toolchain (install via `rustup`)
-   **OS Dependencies:** Follow the [Tauri prerequisites guide](https://v2.tauri.app/start/prerequisites/) for your specific OS.

### Development Setup

1.  **Clone the repository:**
    ```bash
    git clone https://github.com/side-quests/OpenTokenMonitor.git
    cd OpenTokenMonitor
    ```

2.  **Install dependencies:**
    ```bash
    npm install
    ```

3.  **Run the development server:**
    ```bash
    npm run tauri dev
    ```
    This will launch the application in a live-reloading development window.

### Build From Source

You can build the application for your local platform with the following command:

```bash
npm run tauri build
```

The compiled application will be located in `src-tauri/target/release/bundle/`.

## 📄 License

This project is licensed under the MIT License. See the [LICENSE](./LICENSE) file for details.
