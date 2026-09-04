# Release Checklist

CI validates the source (`cargo fmt`, `clippy`, tests, `tsc && vite build`) but
deliberately skips `tauri build`. Nothing automated ever opens the artifact a
user downloads, so this pass is the only thing standing between a bad bundle and
your users.

Run it on a **clean VM or a machine that has never run the app**, per platform.
A dev machine already has `~/.claude`, a populated SQLite store, and a
Gatekeeper exception — all three hide the bugs this pass is looking for.

## Before tagging

- [ ] `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`
- [ ] `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings -A unused-mut`
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml`
- [ ] `npm run build`
- [ ] Version is identical in `package.json`, `src-tauri/tauri.conf.json`,
      `src-tauri/Cargo.toml`, `src/constants/appMeta.ts`, and the README badge
- [ ] Pricing review date in `src-tauri/src/pricing.rs` is current, and the
      rates still match each provider's public page
- [ ] README screenshots match the UI you are about to ship

## Per platform, on a clean machine

### Install

- [ ] Artifact downloads and installs from the GitHub release page
- [ ] **macOS:** double-click opens the app with no Gatekeeper warning.
      Verify: `spctl -a -t exec -vv /Applications/OpenTokenMonitor.app`
      (expect `accepted` / `source=Notarized Developer ID`)
- [ ] **Windows:** installer runs with no SmartScreen interstitial and no UAC
      prompt (`installMode` is `currentUser`)
- [ ] **Linux:** AppImage is executable and launches; `.deb` installs cleanly

### First run, no credentials

This is the state most Product Hunt visitors will see first.

- [ ] App launches without crashing when `~/.claude`, `~/.codex`, and the
      Antigravity log directory are all absent
- [ ] All three providers render with a `waiting` health badge, not an error
      screen and not a blank window
- [ ] No unhandled error appears in the console

### With credentials

- [ ] Claude gauges populate from the OAuth API (5-hour and 7-day at minimum)
- [ ] The Opus window renders even at 0% — it should not vanish after a reset
- [ ] Codex and Antigravity populate from local logs
- [ ] Per-model cost breakdown shows **versioned** model ids
      (`claude-opus-4-5`, not `claude-opus`) and non-zero costs
- [ ] Spot-check one cost figure by hand against `pricing.rs`
- [ ] Quit and relaunch: snapshots persist from SQLite

### Behaviour over time

- [ ] Leave it running ~10 minutes at the default 2-minute cadence. Gauges must
      not drop to 0% or flicker between values. If they do, capture the log —
      that is the failure mode fixed in v0.3.6 regressing.
- [ ] Widget mode toggles and resizes correctly
- [ ] Tray icon and tooltip update

### Privacy claim

The README says nothing leaves the device. Verify it, don't assume it.

- [ ] With a network monitor (Little Snitch, `lsof -i`, Wireshark), confirm the
      only outbound connections are `api.anthropic.com`, `chatgpt.com`,
      `auth.openai.com`, and — only if the updater is configured —
      `github.com` / `objects.githubusercontent.com`
- [ ] No request to `fonts.googleapis.com` or `fonts.gstatic.com`
      (fonts are bundled in `public/fonts/`)

## Updater

- [ ] If `plugins.updater.pubkey` is still `PUBKEY_PLACEHOLDER`: confirm the app
      makes **no** update request at all
- [ ] If a real key is configured: confirm `latest.json` is attached to the
      published release and lists every platform you shipped
- [ ] Best-effort: install the previous version, publish this one, confirm the
      in-app update banner appears and the install completes

## After publishing

- [ ] Download each artifact from the public release page and confirm checksums
      match what CI built
- [ ] Open the release page as a logged-out user — confirm nothing is missing
      for any advertised platform
