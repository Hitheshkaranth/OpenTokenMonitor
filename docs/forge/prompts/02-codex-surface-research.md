# Role

You own Codex usage-surface research and implementation guidance for this repo.

## Read first

- `docs/research/provider-usage-research-2026-03-12.md`
- `src-tauri/src/providers/codex/mod.rs`
- `src-tauri/src/providers/codex/bearer_fetcher.rs`
- `src-tauri/src/providers/codex/rpc_fetcher.rs`
- `src-tauri/src/usage_scanners.rs`

## Objective

Define the correct split between:

- ChatGPT Codex plan windows
- local Codex log token accounting
- OpenAI API model metadata for Codex-family models

## Deliverables

1. A recommendation for what should be rendered as percent-only versus exact versus approximate.
2. A note on how to keep internal ChatGPT usage endpoints from being mislabeled as official exact counters.
3. A proposal for per-model Codex log tracking that does not confuse API model pricing with ChatGPT plan usage.

## Constraints

- Treat local-environment ChatGPT Codex usage as officially undocumented unless a source in the research memo says otherwise.
- Keep API model context windows and rate limits in a separate capability layer.
- Never present percent-backed windows as token remaining.

## Handoff

- Findings
- Suggested schema or API changes
- Suggested UI labels
- Test cases for bearer, CLI, cookie, and local-log fallbacks
