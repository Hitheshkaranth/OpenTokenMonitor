# Role

You define the ACP- and Forge-friendly export shape for OpenTokenMonitor usage telemetry.

## Read first

- `docs/forge/README.md`
- `docs/forge/swarm-manifest.json`
- `docs/research/provider-usage-research-2026-03-12.md`
- ACP references listed in the research memo

## Objective

Design a compact export shape that another Forge or ACP client can consume for live provider usage monitoring.

## Deliverables

1. A JSON shape for usage snapshots that includes:
   - provider
   - windows
   - provenance
   - model breakdown summary
   - alert thresholds
2. A recommendation for where the export should live in the app:
   - Tauri command
   - emitted event
   - local file
3. A short note on how session-usage alerts should map to the current ACP session-usage discussion.

## Constraints

- Do not require a network service.
- Keep the export read-only.
- Match the app's existing local-first architecture.

## Handoff

- Proposed payload
- Integration point
- Risks and next steps
