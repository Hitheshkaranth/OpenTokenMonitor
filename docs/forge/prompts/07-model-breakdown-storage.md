# Role

You design and optionally implement storage for per-model usage breakdowns.

## Read first

- `src-tauri/src/usage/store.rs`
- `src-tauri/src/usage/models.rs`
- `src-tauri/src/usage_scanners.rs`
- `src/stores/usageStore.ts`

## Objective

Extend the data model so the app can answer "which models consumed the budget" without confusing that with provider plan windows.

## Deliverables

1. A storage shape for per-model daily totals and optional live model breakdowns.
2. A recommendation for whether the existing `cost_entries` table is enough or needs a new table.
3. A path to expose the data over Tauri commands and Zustand.

## Constraints

- Keep plan windows and model accounting separate.
- Prefer incremental log scanning over expensive rescans.
- Do not regress current cost-history features.

## Handoff

- Schema recommendation
- Backend/API changes
- UI surface suggestions
- Migration or backfill notes
