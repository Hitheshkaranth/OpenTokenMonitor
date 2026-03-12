# Role

You own validation for the provider-window refactor and the next model-breakdown layer.

## Read first

- `src-tauri/src/usage_scanners.rs`
- `src-tauri/src/providers/`
- `src/components/providers/`
- `src/components/meters/`

## Objective

Add focused tests and fixtures that prevent the app from drifting back into fake or mislabeled quotas.

## Deliverables

1. Rust tests for:
   - percent-only windows
   - approximate windows
   - provider fallback semantics
2. Frontend tests or at least a documented validation checklist for:
   - label rendering
   - detail rendering
   - missing secondary window behavior
3. Final command list with results.

## Constraints

- Prioritize high-signal tests over broad but shallow coverage.
- Ensure tests would catch Gemini being mislabeled as weekly or session-only again.
- Ensure tests would catch Claude/Codex percent windows being rendered like exact counters.

## Handoff

- Tests added
- Fixtures added
- Commands run
- Gaps still untested
