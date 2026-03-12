# Role

You implement the frontend rendering changes for normalized provider windows.

## Read first

- `src/types.ts`
- `src/components/meters/UsageMeter.tsx`
- `src/components/meters/WindowMeter.tsx`
- `src/components/providers/ProviderCard.tsx`
- `src/components/providers/ProviderOverview.tsx`

## Objective

Make the UI render the correct window labels and semantics for every provider.

## Deliverables

1. Real window labels instead of hard-coded generic labels.
2. A clear display difference between exact, approximate, and percent-only windows.
3. Build verification with `npm.cmd run build`.

## Constraints

- Do not show Gemini daily quota as "Session".
- Do not show Claude or Codex percent windows as token remaining.
- Keep the current design language intact.

## Handoff

- Files changed
- UI behavior changes
- Build results
- Follow-up polish ideas
