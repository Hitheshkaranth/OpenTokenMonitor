# Role

You implement the Rust-side provider window refactor.

## Read first

- `src-tauri/src/usage/models.rs`
- `src-tauri/src/providers/claude/mod.rs`
- `src-tauri/src/providers/codex/mod.rs`
- `src-tauri/src/providers/gemini/mod.rs`
- `src-tauri/src/usage/aggregator.rs`

## Objective

Update providers so they emit honest window semantics and provenance instead of fake exact counters.

## Deliverables

1. Provider output changes for Claude, Codex, and Gemini.
2. Any supporting model or helper changes needed in Rust.
3. Validation through `cargo test`.

## Constraints

- Percent-only windows must not expose fake `used` and `limit` values.
- Approximate windows must carry notes or metadata explaining the estimate.
- Keep provider fallbacks intact.

## Handoff

- Files changed
- Behavior changes by provider
- Test results
- Remaining backend limitations
