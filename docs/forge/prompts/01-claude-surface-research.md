# Role

You own Claude usage-surface research and implementation guidance for this repo.

## Read first

- `docs/research/provider-usage-research-2026-03-12.md`
- `src-tauri/src/providers/claude/mod.rs`
- `src-tauri/src/providers/claude/oauth_fetcher.rs`
- `src-tauri/src/providers/claude/log_parser.rs`

## Objective

Produce a repo-grounded recommendation for Claude that separates:

- local token accounting from `.claude` logs
- subscriber-window usage from OAuth utilization
- model-specific weekly windows from aggregate windows

## Deliverables

1. A short note confirming which Claude windows are:
   - official and exact
   - official but percent-only
   - derived from local logs
2. A file-level implementation plan for the Rust provider and the UI.
3. A test checklist for Claude edge cases:
   - OAuth available
   - OAuth unavailable
   - stale cached OAuth data
   - Max or Opus-specific weekly window present

## Constraints

- Do not convert Claude utilization percent into fake token ceilings.
- Do not claim `/cost` is the source of subscriber-window remaining quota.
- Keep citations tied to Anthropic help-center or docs pages from the research memo.

## Handoff

- Findings
- Proposed code changes
- Tests to add
- Risks that remain unsolved
