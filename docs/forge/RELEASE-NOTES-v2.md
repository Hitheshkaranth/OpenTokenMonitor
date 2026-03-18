# Release Notes: Provider Tracking Refactor (v2.0)

Date: 2026-03-13

This update introduces a major refactor to how OpenTokenMonitor tracks usage for Claude, Codex, and Gemini. The focus of this release is **Honest Tracking**: we've moved away from "one-size-fits-all" counters toward provider-specific semantics that respect what each API and local log surface actually exposes.

## Key Changes

### 1. Normalized Accuracy Model
We no longer pretend every provider exposes an exact token count. The UI now distinguishes between three levels of precision:
- **Exact:** Real counters from official provider surfaces (e.g., Gemini rate limits).
- **Approximate:** Derived from local CLI logs (`.claude`, `.codex`).
- **Percent-Only:** Rolling subscription windows where only utilization percent is known (e.g., Claude Pro 5h/7d windows).

### 2. Provider-Specific Windows
- **Claude:** Correctly labels `5h Window` and `7d Window`. Subscription windows are marked as "Percent-based" to avoid misleading token estimates.
- **Codex:** Separates ChatGPT Plan windows from Codex API model accounting. Per-model token/cost tracking is now surfaced from local logs.
- **Gemini:** Prioritizes `Daily` and `RPM` (Session) windows. Weekly views are explicitly labeled as "Derived 7d usage" to reflect that they are local analytics, not official hard limits.

### 3. Model Mix & Alerts
- **Model Mix:** Provider cards now show a breakdown of which models are consuming tokens (e.g., `claude-3-7-sonnet` vs `claude-3-5-haiku`).
- **Threshold Alerts:** Automated alerts are now generated at **75%**, **90%**, and **95%** utilization. These are visible in both the Provider Cards and the Diagnostics panel.

### 4. Data Provenance
Every snapshot now carries a "Provenance" tag:
- `official`: Sourced from a public API.
- `internal`: Sourced from an internal app/CLI surface.
- `derived_local`: Computed locally from usage logs.

## Technical Improvements
- **Rust Backend:** Refactored `usage_scanners.rs` to handle incremental log parsing for Claude and Codex.
- **Storage:** Per-model usage is now aggregated and stored in a local SQLite database (`usage.db`).
- **Export:** Added a new Tauri command `export_usage_report` that generates an ACP-compatible JSON payload for use with other agents (like Forge).

## Limitations & Operator Caveats
- **Gemini Weekly:** There is no official Gemini weekly limit. Any 7-day display is a local synthesis and should be used for trend analysis only.
- **Claude/Codex Subscriber Windows:** These windows only return utilization percent. If you see "90% used," this is the provider's official signal; the app will not "guess" the remaining tokens.
- **Token Counting:** Token counts are estimates based on standard provider pricing tables and local log analysis. They may differ slightly from official billing statements.
