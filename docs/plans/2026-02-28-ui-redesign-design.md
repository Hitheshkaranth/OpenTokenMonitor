# UI Redesign Design — 2026-02-28

## Problem Statement

The widget has several issues blocking usability:
1. Window starts hidden (`visible: false`) — user cannot see it
2. Logo not displayed — header shows a generic gradient div, not the actual `open_token_monitor_icon.png`
3. Widget too large — 420×580px; should be ~360×320px (roughly half the area)
4. Typography too small — many labels at 8–9px, low contrast
5. Apple Liquid Glass aesthetic incomplete — basic backdrop-filter only, no shimmer, rim, or provider accent glows

## Approved Design

### Window Config (`tauri.conf.json`)
- `width: 360`, `height: 320`
- `visible: true` (show on launch)
- `decorations: false`, `transparent: true`, `alwaysOnTop: true` (unchanged)

### Layout (top to bottom, 320px total)

| Region | Height | Content |
|--------|--------|---------|
| Header | 44px | 20px logo + "OpenTokenMonitor" title · Refresh + Settings buttons |
| Summary bar | 44px | `$XX.XX` cost (left) · `X.XM tokens` (right) · center divider |
| Tab bar | 36px | Live Activity · Stats · Trends pill tabs |
| Content area | ~164px | Scrollable, fills remaining space |
| Footer | 32px | Provider dot indicators + status text |

### Apple Liquid Glass Visual System

**Container**
```css
background: linear-gradient(160deg, rgba(20,14,40,0.82) 0%, rgba(8,8,16,0.85) 100%);
backdrop-filter: blur(36px) saturate(200%);
border: 1px solid rgba(255,255,255,0.10);
box-shadow:
  inset 0 1px 0 rgba(255,255,255,0.15),   /* top rim shimmer */
  0 8px 32px rgba(0,0,0,0.85);             /* outer depth */
border-radius: 18px;
```

**Shimmer overlay** (pseudo-element)
```css
::before {
  background: linear-gradient(120deg, rgba(255,255,255,0.06) 0%, transparent 60%);
  pointer-events: none;
  position: absolute; inset: 0;
  border-radius: inherit;
}
```

**Provider accent glows** (per card)
- Claude / Anthropic: `box-shadow: 0 0 18px rgba(217,119,87,0.27)`
- OpenAI: `box-shadow: 0 0 18px rgba(16,163,127,0.27)`
- Google Gemini: `box-shadow: 0 0 18px rgba(66,133,244,0.27)`

**Typography**
- Body: Outfit, 13px minimum
- Data values: JetBrains Mono, 13–15px
- Labels: 10px minimum (no 8–9px sizes)
- Text colors: `--text-primary: #EDEEF2`, `--text-secondary: #9B9DAA`

**Logo**
- `<img src="/open_token_monitor_icon.png" width="20" height="20" style="border-radius:5px" />`
- Used in header and as tray icon source

**Tab pills**
- Active: `background: rgba(255,255,255,0.10)`, `border-left: 2px solid <providerColor>`
- Inactive: transparent, `color: var(--text-secondary)`

## Implementation Approach

Option A: CSS-only (no new dependencies)
- Rebuild `index.css` with new glass variables and shimmer
- Update `App.tsx`: replace gradient div with logo img, fix font sizes, add glow classes
- Update `tauri.conf.json`: new dimensions, `visible: true`
- Copy logo to `public/` so Vite serves it at `/open_token_monitor_icon.png`
