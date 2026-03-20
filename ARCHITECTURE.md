# OpenTokenMonitor Architecture

This document explains how the current codebase is wired together and where to
start when changing behavior. It is written for contributors who need a quick,
accurate map of the runtime instead of a marketing overview.

## 1. System Shape

OpenTokenMonitor is a Tauri desktop app with:

- a React + TypeScript frontend in `src/`
- a Rust backend in `src-tauri/src/`
- a SQLite persistence layer managed by the Rust backend
- provider adapters that combine local CLI artifacts with live authenticated fetches

The important design choice is that the frontend does not talk to provider APIs
directly. The React app talks to the Rust backend through Tauri commands, and
the Rust backend owns provider fetch logic, persistence, background refreshes,
and filesystem watching.

## 2. Runtime Flow

### App startup

1. Tauri starts the Rust backend from `src-tauri/src/main.rs`.
2. `src-tauri/src/lib.rs` builds the Tauri app, initializes plugins, opens the
   SQLite-backed `UsageStore`, registers providers, configures the tray, starts
   file watchers, and starts the poll scheduler.
3. React mounts `src/App.tsx`.
4. `App.tsx` activates:
   - `useGlassTheme` to apply the current theme
   - `useUsageData` to bootstrap usage snapshots and subscribe to backend events
   - `useProviderStatus` to poll provider availability
5. Zustand stores in `src/stores/` become the shared source of truth for the UI.

### Data refresh

The normal refresh path looks like this:

1. React calls a store action such as `refreshAll()` in `src/stores/usageStore.ts`.
2. The store calls a Tauri command through `invoke(...)`.
3. Rust receives the command in `src-tauri/src/lib.rs`.
4. The command delegates to `src-tauri/src/usage/aggregator.rs`.
5. The aggregator asks a provider implementation for fresh usage.
6. The backend writes snapshots and cost history into SQLite.
7. Rust emits a `usage-updated` event.
8. `useUsageData` listens for that event and upserts the updated snapshot into the frontend store.

### Background updates

Two backend systems keep the UI current even when the user does not press refresh:

- `src-tauri/src/watchers/poll_scheduler.rs`
  Periodically triggers refreshes based on the configured cadence.
- filesystem watchers under `src-tauri/src/watchers/`
  React to changes in local CLI artifacts and re-run provider refreshes.

## 3. Frontend Structure

### `src/App.tsx`

`App.tsx` is the frontend orchestrator. It is responsible for:

- deciding whether the app is in widget mode or full dashboard mode
- managing current page selection
- kicking off initial fetches
- coordinating refreshes across providers
- syncing desktop-only settings such as launch-on-startup
- resizing the Tauri window when widget mode changes

If you need to understand "what happens when the app opens?", start here.

### `src/stores/usageStore.ts`

This is the main frontend bridge to the backend. It stores:

- current snapshots
- trends
- model breakdowns
- recent activity
- provider status results
- generated alerts and reports

Every action in this store is intentionally thin:

- call a backend command
- normalize the returned shape into per-provider maps
- update Zustand state

If backend data is in the app but not on screen, this is usually the first place to inspect.

### `src/hooks/useUsageData.ts`

This hook handles usage bootstrap and live synchronization:

- tries `refreshAll()` first so the app prefers fresh backend data
- falls back to cached snapshots when refresh fails
- fetches recent activity for every provider
- listens for the backend `usage-updated` event and merges incoming snapshots

### `src/hooks/useProviderStatus.ts`

This hook polls provider availability independently from usage snapshots. That
separation lets the UI distinguish:

- "provider is reachable but usage is still loading"
- "provider is only partially available"
- "provider is unavailable"

### `src/components/`

The main UI surfaces are grouped by responsibility:

- `components/layout/`
  Sidebar, widget mode, widget activity surface
- `components/providers/`
  Overview cards and full provider detail screens
- `components/settings/`
  Settings and About panels
- `components/meters/`
  Circular/widget gauges, reset countdowns, and usage meters
- `components/states/`
  Empty, loading, error, and diagnostics states

## 4. Backend Structure

### `src-tauri/src/lib.rs`

This file is the backend composition root. It contains:

- the shared `AppState`
- Tauri command handlers
- startup/shutdown behavior
- tray setup
- autostart support
- event emission after refreshes
- scheduler and watcher wiring

This is the best place to start when a frontend `invoke(...)` call is failing.

### `src-tauri/src/providers/`

Provider implementations live here. Each provider conforms to the `UsageProvider`
trait in `src-tauri/src/providers/mod.rs`:

- `fetch_usage`
- `fetch_cost_history`
- `check_status`

`registry.rs` is the one place where providers are registered. Adding a new
provider always means:

1. implement the trait
2. register it in `ProviderRegistry::new()`
3. update any frontend provider enums or metadata maps

### `src-tauri/src/usage/aggregator.rs`

The aggregator is deliberately simple. It does not own provider-specific logic.
Its job is to:

- ask a provider for fresh usage
- persist the snapshot
- persist optional cost history
- aggregate provider errors when running `refresh_all`

### `src-tauri/src/usage/store.rs`

This module owns SQLite persistence. It is the durable source for:

- latest snapshots
- cost history
- model breakdown queries
- usage trend queries

When the UI asks for cached historical data, the answer comes from here.

### `src-tauri/src/usage_scanners.rs`

This module scans local recent CLI activity and turns that into the prompt
history shown in the widget and provider detail pages.

### `src-tauri/src/watchers/`

These modules keep local-file-driven providers reactive:

- `file_watcher.rs`
  Watches the relevant directories and triggers provider refreshes
- `poll_scheduler.rs`
  Owns the repeatable timer used for cadence-based refreshes

## 5. Data Ownership

The backend is authoritative for persisted usage data.

- Frontend state is a projection for rendering and interaction.
- Backend SQLite is the durable store.
- Provider modules are the only layer that should know how Claude, Codex, or Gemini are fetched.

This separation matters because it keeps provider quirks out of the React tree.

## 6. Common Change Paths

### Add a new field to a usage snapshot

1. Update the Rust model in `src-tauri/src/usage/models.rs`
2. Update provider fetchers to fill the field
3. Update persistence queries if the field must be stored
4. Update the TypeScript mirror in `src/types.ts`
5. Render the field in the relevant React component

### Change provider refresh behavior

1. Start in `src-tauri/src/usage/aggregator.rs`
2. Check provider logic in `src-tauri/src/providers/<provider>/`
3. Check event emission and scheduler wiring in `src-tauri/src/lib.rs`
4. Confirm the frontend hook/store path in `useUsageData.ts` and `usageStore.ts`

### Change widget behavior

1. `src/components/layout/WidgetMode.tsx`
2. `src/components/layout/WidgetActivityView.tsx`
3. `src/components/meters/WidgetGauge.tsx`
4. `src/styles/sidebar.css`
5. `src/App.tsx` if the window size must change

## 7. Best Entry Points For Reading

If you are new to the repo, read in this order:

1. `README.md`
2. `ARCHITECTURE.md`
3. `src/App.tsx`
4. `src/stores/usageStore.ts`
5. `src/hooks/useUsageData.ts`
6. `src-tauri/src/lib.rs`
7. `src-tauri/src/providers/mod.rs`
8. `src-tauri/src/usage/aggregator.rs`

That sequence gives the fastest path from "what is this app?" to "where do I make the change?"
