# Implementation Brief: Replace the Gemini provider with Antigravity CLI

**Audience:** the coding agent (Antigravity) that will implement this.
**Author:** orchestration/architecture pass — do not deviate from the touchpoint
list without flagging.
**Goal:** OpenTokenMonitor currently monitors **Gemini CLI** as one of three
providers (`claude`, `codex`, `gemini`). Replace the Gemini provider end-to-end
with an **Antigravity** provider — same registry-driven architecture, new data
sources, new icon, new brand. The other two providers (`claude`, `codex`) are
untouched.

Why now: Google is sunsetting the standalone Gemini CLI for individual / AI Pro /
AI Ultra tiers on **2026-06-18**, folding it into **Antigravity** (the multi-agent
IDE + `agy` CLI). Monitoring Antigravity is the forward-compatible target.

---

## 0. Ground rules

- This is a **replacement**, not an addition. After this change there is **no
  `gemini` provider** anywhere — enum, modules, assets, frontend types, CSS, copy.
- Keep the existing `UsageProvider` trait contract exactly. The Antigravity
  provider must implement the same 6 trait methods the Gemini one did
  (`id`, `descriptor`, `fetch_usage`, `fetch_cost_history`, `check_status`,
  `compute_auth_state`). See `src-tauri/src/providers/mod.rs`.
- Do **not** invent a new window-type or unit enum variant — the model already has
  what we need (`WindowType::FiveHour`, `WindowType::Weekly`, `WindowType::Daily`,
  `UsageUnit::Requests`). See `src-tauri/src/usage/models.rs`.
- **Verify data paths against a real Antigravity install before shipping.** Section
  6 lists best-known candidate paths and the live-quota loopback; confirm them on
  the dev machine (`~/Library/Application Support/Antigravity/…`) rather than
  trusting this doc blindly. The fetch tiers degrade gracefully if a path is wrong,
  but the local-log scanner needs a correct directory to produce data.
- Respect repo conventions: match the surrounding Rust/TS style, keep
  `lib.rs`/`commands.rs` thin, keep all provider quirks inside the provider module.
- **Do not git commit.** The repo owner reviews diffs personally.

---

## 1. What Antigravity exposes (data model)

From Antigravity's public-preview behavior and the community `antigravity-usage`
CLI, there are three ways to read quota — they map 1:1 onto the three tiers the
Gemini provider already used (OAuth fetch → live CLI → local-log fallback):

| Tier | Source | Analogue in current Gemini provider |
|------|--------|-------------------------------------|
| A. Live local | Antigravity **Language Server loopback** running inside the IDE (local HTTP port) — exact, includes per-model quota | `stats_parser::fetch_stats()` (the `gemini --stats --json` path) |
| B. Cloud | **Google Cloud Code API** with the user's Google **OAuth** token | `oauth_fetcher::fetch_quota()` |
| C. Local-log fallback | Parse Antigravity session/agent logs on disk for derived/approximate usage | `scan_gemini_*` in `usage_scanners.rs` |

**Quota / reset model (plan-dependent — surface plan in `PlanInfo`):**

- **AI Pro / AI Ultra:** rolling **5-hour** window, priority access. → `WindowType::FiveHour`.
- **Free:** **weekly** refresh. → `WindowType::Weekly`.
- **CLI via OAuth:** hard cap of **~200 requests / 24h rolling** window. → `WindowType::Daily`.
- Limits are **aggregated across model families** (Pro / Flash) but Antigravity now
  also exposes **per-model** quota — use per-model when available, else aggregate.
- Quota responses are **cached ~5 min** upstream; our own poll cadence already
  handles refresh, so don't add a second cache layer in the provider.

**Recommended window set for `fetch_usage` (two windows, matching the Gemini shape):**

1. **Primary** — `WindowType::FiveHour`, `UsageUnit::Requests`: the Pro 5-hour
   rolling quota (the window users actually hit). For Free-tier users, substitute
   `WindowType::Weekly`.
2. **Secondary** — `WindowType::Daily`, `UsageUnit::Requests`: the 24h-rolling CLI
   request cap (~200). This is the second meter the UI shows.

Pick limits from the live/OAuth source when present; fall back to the constants
below for the local-log path. Mirror the Gemini provider's `exact` vs
`approximate` discipline: live/OAuth → `UsageWindow::exact`; derived-from-logs →
`UsageWindow::approximate(... note)`.

```
const ANTIGRAVITY_CLI_DAILY_REQUEST_CAP: u64 = 200;   // 24h rolling, OAuth CLI
const ANTIGRAVITY_FIVE_HOUR_REQUEST_LIMIT: u64 = ???;  // confirm from live API; gate behind plan
```

> The 5-hour numeric limit is not publicly fixed and varies by plan. Read it from
> the live/OAuth payload; only fall back to a constant if the API omits it, and
> mark that window `approximate` when you do.

---

## 2. Architecture (unchanged in shape)

Nothing about the app's wiring changes — only the provider identity and its data
sources. The flow stays:

```
React store (invoke) → commands.rs → aggregator.rs → AntigravityProvider.fetch_usage()
        ↑ usage-updated event ← SQLite persist ← snapshot
```

The new provider lives at `src-tauri/src/providers/antigravity/` and is the only
place that knows how Antigravity is read. Registry registration is the single
wiring point (see `ARCHITECTURE.md` §4 "Adding a new provider").

---

## 3. Backend change list (Rust)

Work module-by-module. File paths are relative to repo root.

### 3.1 `src-tauri/src/usage/models.rs` — the enum (root of the rename)
- Rename `ProviderId::Gemini` → `ProviderId::Antigravity`.
- `as_str()`: `Self::Antigravity => "antigravity"`.
- `all()`: `[Claude, Codex, Antigravity]`.
- The `#[serde(rename_all = "lowercase")]` means the wire value becomes
  `"antigravity"` — the frontend `ProviderId` union (§4) must match exactly.
- No other variant/struct in this file needs changes (`WindowType::FiveHour`,
  `Weekly`, `Daily`, `UsageUnit::Requests` already exist).

### 3.2 New module `src-tauri/src/providers/antigravity/`
Create, modeled on `providers/gemini/`:
- `mod.rs` — `AntigravityProvider` implementing `UsageProvider`. Three-tier
  `fetch_usage` per §1 (live loopback → OAuth Cloud Code API → local-log scan).
  Replace the Gemini quota constants/`next_daily_reset` with the Antigravity
  windows (§1). `check_status` should detect: (a) the live language-server port
  open, (b) Antigravity config/creds on disk, (c) local logs present — Active if
  any, else Waiting (mirror the Gemini `check_status` structure).
- `descriptor.rs` — `display_name: "Antigravity"`, `id: ProviderId::Antigravity`,
  `brand_color`: see §5 (confirm exact hex).
- `live_fetcher.rs` (replaces `stats_parser.rs`) — connect to the local
  Antigravity Language Server loopback; parse its quota JSON into the two windows.
  Gate behind `ctx.allow_cli_strategy` like the Gemini stats path was.
- `oauth_fetcher.rs` — call the Google Cloud Code API with the user's OAuth token
  to read quota. Keep the same `fetch_quota`-style signature/return shape the
  Gemini one had so `mod.rs` reads cleanly. **Do not perform an OAuth refresh /
  token-rotation smoke test against the user's real Google credentials** — read
  only; rotating the refresh token would silently break the user's Antigravity/CLI
  login.
- Delete the whole `providers/gemini/` directory.

### 3.3 `src-tauri/src/providers/mod.rs`
- `pub mod gemini;` → `pub mod antigravity;`.

### 3.4 `src-tauri/src/providers/registry.rs`
- `use ...gemini::GeminiProvider;` → `...antigravity::AntigravityProvider;`.
- Replace the `gemini` local + `providers.insert(gemini.id(), gemini)` lines with
  the Antigravity equivalent.

### 3.5 `src-tauri/src/usage_scanners.rs` (~92 KB — the big one)
This file has the Gemini local-log scanner: `GeminiLogFile`, `GeminiDailyUsagePoint`,
`GeminiModelDailyUsagePoint`, `GeminiContribution`, `GeminiFileCache`,
`GeminiScannerCache`, `GEMINI_CACHE`, `scan_gemini_daily_usage`,
`scan_gemini_model_daily_usage`, `scan_gemini_recent_activity`,
`discover_gemini_log_files`, `discover_gemini_chat_files`,
`collect_gemini_recent_entries`, `read_gemini_chat_model_lookup`,
`insert_gemini_chat_models_from_session`, `extract_gemini_user_prompt`,
`extract_gemini_assistant_text`, `normalize_gemini_model`, and the
`refresh_gemini`/`gemini_daily`/`gemini_model_daily` impls (lines ~137–1440).
- Rename the symbols `Gemini*`→`Antigravity*` / `gemini`→`antigravity`.
- **Repoint discovery** from `~/.gemini/tmp` + `~/.gemini` chat files to the
  Antigravity session/log directory (§6). The chat/session JSON shape differs —
  rewrite `discover_*`, `collect_*_recent_entries`, `extract_*_prompt/text`,
  and the chat-model lookup to match Antigravity's actual on-disk format. This is
  the highest-effort part; budget for it and verify against real files.
- Keep the incremental-cache pattern (`*FileCache`, `OnceLock<Mutex<…>>`) intact —
  it's a performance feature, not Gemini-specific.
- Update the `match provider { ProviderId::Gemini => scan_gemini_recent_activity }`
  arm (~line 445) to the Antigravity arm.

### 3.6 `src-tauri/src/pricing.rs`
- Rename `gemini_rates` / `gemini_cost_usd` → `antigravity_rates` / `antigravity_cost_usd`.
- Keep the underlying Gemini-model rate rows (Antigravity runs Gemini 3 Pro / Flash
  models) but **add/refresh rows for the models Antigravity actually reports**
  (e.g. `gemini-3-pro`, `gemini-3.5-flash`, and any `claude-sonnet-4-5` if
  Antigravity surfaces it). Update the module doc + rate-review date. Run the
  `pricing::tests` after.
- Update the `normalize_gemini_model` callsite name and any model-alias branches.

### 3.7 `src-tauri/src/tray.rs`
- The tray tooltip (~lines 35–48) hardcodes `Gemini: {:.0}%`. Rename the local
  var and the format string label to `Antigravity`. Consider abbreviating to
  `Antigrav` or `AG` if `"Antigravity"` makes the tooltip too wide — your call,
  but keep all three providers visible.

### 3.8 `src-tauri/src/watchers/file_watcher.rs`
- Line ~38: the watched path `(ProviderId::Gemini, home.join(".gemini").join("tmp"))`
  → `(ProviderId::Antigravity, <antigravity session/log dir from §6>)`.

### 3.9 `src-tauri/src/alerts.rs`
- Doc comment (~line 12) and the test fixture (~line 109,
  `snapshot_with_utilization(ProviderId::Gemini, 96.0)`) → Antigravity.

### 3.10 `src-tauri/Cargo.toml`
- Only if it names a gemini-specific feature/dep (grep hit). Likely just a comment;
  update if present, otherwise no-op.

---

## 4. Frontend change list (React / TS)

The string `"gemini"` is the wire value, so every map keyed by `ProviderId` must
switch its key to `"antigravity"`. Files (all under `src/`):

- **`types.ts`** — `ProviderId = 'claude' | 'codex' | 'antigravity'`. This drives
  the type-check across everything below.
- **`stores/settingsStore.ts`** — `enabledProviders`, `apiKeys` default maps:
  `gemini` key → `antigravity`.
- **`stores/usageStore.ts`** — all per-provider map literals (`snapshots`,
  `costHistory`, `trends`, `modelBreakdowns`, `recentActivity`, `statuses`,
  `alerts`, `authStates`), the `providers: ProviderId[]` array, and the two
  `.filter(p => p.provider === 'gemini')` calls.
- **`App.tsx`** — the three `['claude','codex','gemini']` literals (lines ~65, 78,
  123), the `snapshots.gemini?.fetched_at` dep (~104), and the loading-guard
  `!snapshots.gemini` (~146).
- **`utils/usageWindows.ts`** — `displayWindows` special-cases
  `snapshot.provider !== 'gemini'` (reorders the two windows by soonest reset).
  Keep the behavior, switch the literal to `'antigravity'` (the 5-hour vs 24h
  windows still benefit from "show soonest-resetting first").
- **`components/providers/ProviderLogo.tsx`** — `srcByProvider.gemini` →
  `antigravity: '/providers/antigravity-icon.png'`; add an `antigravity` entry to
  `widgetCoreOptics` (start from the gemini values `{scale:1.08,x:0,y:-0.1}` and
  tune for the new icon).
- **`components/settings/SettingsPage.tsx`** — `providers` array, the label map
  (`'Gemini'`→`'Antigravity'`), the api-key-placeholder map, and the brand RGB
  triplet map (`'66 133 244'` → the Antigravity RGB, §5).
- **`components/layout/WidgetActivityView.tsx`** & **`WidgetMode.tsx`** — the
  `providerMeta`/`meta` records (label + `tint`) and the `providers` arrays. Note
  `tint` is typed as a literal union `'claude'|'codex'|'gemini'` — widen/rename it
  to `'antigravity'` and update the matching CSS classes (§4.1).
- Sweep the remaining grep hits for any stray `gemini` label/aria/copy in:
  `components/layout/Sidebar.tsx`, `components/glass/GlassPanel.tsx`,
  `components/meters/UsageMeter.tsx`, `components/meters/WindowMeter.tsx`,
  `components/projects/ProjectOverview.tsx`, `components/providers/OverviewCard.tsx`,
  `components/providers/ProviderCard.tsx`, `components/providers/ProviderOverview.tsx`,
  `components/settings/AboutPanel.tsx`, `components/states/DiagnosticsPanel.tsx`,
  `hooks/useKeyboardShortcuts.ts`, `hooks/useProviderStatus.ts`,
  `hooks/useUsageData.ts`.

### 4.1 CSS — `src/styles/glass.css` & `src/styles/sidebar.css`
- `--gemini-tint` → `--antigravity-tint` (new RGBA from §5).
- `.glass-gemini`, `.nav-pill-tint-gemini` (+`::before`), `.widget-card.accent-gemini::after`,
  `.widget-provider-card.glass-gemini`, `.widget-activity-panel.glass-gemini`,
  `.overview-card-v2.glass-gemini { --widget-accent: … }` → `-antigravity` and the
  new brand color / RGB triplet. Rename every class so the `tint`/`glass-*`
  className strings produced in TSX resolve.

---

## 5. Branding & icon

- **Icon asset:** add `public/providers/antigravity-icon.png` (square, transparent
  PNG, ≥256×256, consistent visual weight with the existing
  `claude-ai-icon.png` / `chatgpt-icon.png`). Use the official Antigravity mark.
  Remove the now-unused gemini icons from `public/providers/`
  (`google-gemini-icon.png`, `geminai.webp`). Also drop the copies under
  `dist/providers/` if they're checked in (they're build output — regenerating is
  fine).
- **Brand color:** Antigravity's mark reads as a deep blue/indigo. **Confirm the
  exact hex from the official brand** before finalizing. Until confirmed, use a
  placeholder and flag it:
  - `brand_color` (Rust descriptor) + `.accent`/gradient stops (CSS): e.g. `#4f6bed`
  - RGB triplet for `SettingsPage.tsx` + `--widget-accent` + tint: e.g. `79 107 237`
  - tint RGBA (`glass.css`): e.g. `rgba(79, 107, 237, 0.22)`
  Keep all four representations consistent with whatever final hex is chosen.
- **App tray/window icons** under `src-tauri/icons/` are the *app* icon, not the
  provider — leave them.

---

## 6. Data paths to VERIFY on a real install (do not trust blindly)

Antigravity is a VS Code / Windsurf-family fork; the agent implementing this is
best positioned to confirm its own layout. Confirm and then wire §3.5/§3.8:

- **Live quota (Tier A):** the Antigravity Language Server loopback — find the
  local port it listens on (inspect the running IDE / its logs) and the quota
  endpoint + JSON shape. This is what the community `antigravity-usage` tool calls
  its "local mode."
- **OAuth creds + Cloud Code API (Tier B):** where Antigravity stores the Google
  OAuth token on this OS, and the Cloud Code API quota endpoint. Read-only.
- **Local logs/sessions (Tier C):** the agent/session log directory. Candidates to
  check (macOS): `~/Library/Application Support/Antigravity/User/…`,
  `~/Library/Application Support/Antigravity/logs/…`, and any `~/.antigravity` or
  `~/.codeium`-style dir. This replaces `~/.gemini/tmp`.

Record the confirmed paths in code comments where the Gemini paths used to be, so
the next maintainer isn't guessing.

---

## 7. QC / acceptance checklist (run before handing back)

**Required: build AND run the app — do not hand back on a green typecheck alone.**
Known-good baseline (verified 2026-06-12, pre-change, still showing `gemini`):
`npm run tauri dev` compiles the Rust backend and opens the native window; the
backend logs `Loaded providers: codex…, gemini…, claude…` and live quota fetches
succeed. After your change the same command must:
- [ ] compile clean (the pre-existing `unused_mut` warning in the old
      `gemini/stats_parser.rs` disappears because that file is gone — do not
      introduce new warnings);
- [ ] log `Loaded providers:` with **antigravity** in place of gemini and the new
      brand color;
- [ ] open the window and render (no blank screen — the repo has a prior
      blank-screen-boot regression, so actually look at the window, don't trust the
      log alone);
- [ ] `npm run tauri:build` (release bundle) also succeeds.

Backend:
- [ ] `cargo build` clean; `cargo test --lib` green (esp. `pricing::tests` and the
      provider id test renamed to Antigravity).
- [ ] `grep -ri "gemini" src-tauri/src` returns **zero** hits (except deliberate
      historical notes, if any — there should be none).
- [ ] `ProviderId::all()` returns exactly `[Claude, Codex, Antigravity]`; serde
      wire value is `"antigravity"`.

Frontend:
- [ ] `npm run build` / `tsc` clean — no `ProviderId` exhaustiveness errors.
- [ ] `grep -ri "gemini" src` returns **zero** hits.
- [ ] App boots, sidebar shows **Antigravity** with the new icon and brand tint;
      Claude + Codex unchanged.
- [ ] Settings page lists Antigravity; enable/disable + api-key field work.
- [ ] Tray tooltip shows three providers including Antigravity.

Runtime data:
- [ ] With an Antigravity install present, `check_status` reports Active and at
      least one fetch tier returns real windows (5-hour + 24h request meters).
- [ ] With no install, provider shows Waiting (not Error), like Gemini did.

Docs:
- [ ] `ARCHITECTURE.md` §5 / §7 and `README.md` mentions of Gemini updated to
      Antigravity.

---

## 8. Out of scope / do not touch

- Claude and Codex providers and their assets.
- The `UsageProvider` trait signature, aggregator, store schema, watcher framework.
- App-level Tauri icons.
- Any git commit (owner reviews diffs).
