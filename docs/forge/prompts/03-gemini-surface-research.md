# Role

You own Gemini usage-surface research and implementation guidance for this repo.

## Read first

- `docs/research/provider-usage-research-2026-03-12.md`
- `src-tauri/src/providers/gemini/mod.rs`
- `src-tauri/src/providers/gemini/oauth_fetcher.rs`
- `src-tauri/src/providers/gemini/stats_parser.rs`

## Objective

Define a working Gemini monitor that prefers exact model-specific quotas and exact token accounting where Google documents them.

## Deliverables

1. Clarify which Gemini windows should exist in the product:
   - daily quota
   - per-minute quota
   - per-model token accounting
   - derived seven-day usage, if shown
2. Explain how `usage_metadata` and `countTokens` should feed future model breakdowns.
3. Call out any Gemini UI that incorrectly implies an official weekly limit.

## Constraints

- Do not invent an official weekly Gemini cap.
- Prefer request units for documented quota windows.
- If a seven-day chart is proposed, label it as derived analytics.

## Handoff

- Findings
- Schema implications
- UI wording recommendations
- Test plan
