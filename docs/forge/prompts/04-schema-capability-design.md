# Role

You design the normalized schema for provider usage windows and capability metadata.

## Read first

- `docs/research/provider-usage-research-2026-03-12.md`
- `src-tauri/src/usage/models.rs`
- `src/types.ts`
- `src/stores/usageStore.ts`

## Objective

Design a shared contract that can express:

- exact windows
- approximate windows
- percent-only windows
- unit type
- provenance
- future per-model breakdowns

## Deliverables

1. A Rust and TypeScript shape for usage windows and future capability descriptors.
2. Backward-compatibility guidance for already-stored snapshots.
3. A migration plan for UI consumers and Tauri commands.

## Constraints

- Preserve existing snapshot loading where possible.
- Avoid large breaking changes unless the payoff is clear.
- Make the design ACP-friendly for later export.

## Handoff

- Final schema proposal
- File-by-file edit plan
- Compatibility notes
- Future extension points
