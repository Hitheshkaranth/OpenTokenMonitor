# UI Redesign Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Redesign the OpenTokenMonitor widget to 360×320px with Apple Liquid Glass dark aesthetic, real logo, visible-on-launch window, and readable typography.

**Architecture:** Pure CSS redesign — no new dependencies. Update `tauri.conf.json` for new dimensions and `visible: true`, copy logo to `public/`, rebuild `index.css` with glass variables and shimmer, update `App.tsx` to use the real logo image and fix all font sizes.

**Tech Stack:** React 19, TypeScript, Tauri v2, CSS (no new packages)

---

### Task 1: Fix window config and make it visible

**Files:**
- Modify: `src-tauri/tauri.conf.json`

**Step 1: Update window dimensions and visibility**

Change the window config to:
```json
{
  "label": "main",
  "title": "OpenTokenMonitor",
  "width": 360,
  "height": 320,
  "decorations": false,
  "transparent": true,
  "alwaysOnTop": true,
  "skipTaskbar": true,
  "resizable": false,
  "visible": true
}
```

**Step 2: Verify the file looks correct**

Open `src-tauri/tauri.conf.json` and confirm `width: 360`, `height: 320`, `visible: true`.

**Step 3: Commit**

```bash
git add src-tauri/tauri.conf.json
git commit -m "fix: set window to 360x320 and visible on launch"
```

---

### Task 2: Copy logo to public folder

**Files:**
- Create: `public/open_token_monitor_icon.png` (copy from root)

**Step 1: Copy the logo**

```bash
cp open_token_monitor_icon.png public/open_token_monitor_icon.png
```

Vite serves everything in `public/` at the root path `/`, so the image will be accessible at `/open_token_monitor_icon.png` in the frontend.

**Step 2: Verify**

```bash
ls public/
```
Expected: `open_token_monitor_icon.png` appears in the listing.

**Step 3: Commit**

```bash
git add public/open_token_monitor_icon.png
git commit -m "feat: add app logo to public assets"
```

---

### Task 3: Rebuild index.css with Apple Liquid Glass system

**Files:**
- Modify: `src/index.css`

**Step 1: Replace the entire file with the new design system**

```css
:root {
  --bg-glass: linear-gradient(160deg, rgba(20,14,40,0.82) 0%, rgba(8,8,16,0.85) 100%);
  --bg-card: rgba(255,255,255,0.04);
  --bg-card-hover: rgba(255,255,255,0.07);
  --border-subtle: rgba(255,255,255,0.10);
  --border-rim: rgba(255,255,255,0.15);
  --text-primary: #EDEEF2;
  --text-secondary: #9B9DAA;
  --text-muted: #6B6D7A;
  --claude-primary: #D97757;
  --openai-primary: #10A37F;
  --gemini-primary: #4285F4;
  --font-display: 'Outfit', 'Inter', sans-serif;
  --font-mono: 'JetBrains Mono', 'Fira Code', monospace;
}

* {
  box-sizing: border-box;
  margin: 0;
  padding: 0;
  user-select: none;
  cursor: default;
}

body {
  background: transparent;
  color: var(--text-primary);
  font-family: var(--font-display);
  overflow: hidden;
  height: 100vh;
  width: 100vw;
}

#root {
  height: 100%;
  display: flex;
  flex-direction: column;
}

/* ── Liquid Glass Container ── */
.liquid-glass-container {
  position: relative;
  background: var(--bg-glass);
  backdrop-filter: blur(36px) saturate(200%);
  -webkit-backdrop-filter: blur(36px) saturate(200%);
  border: 1px solid var(--border-subtle);
  border-radius: 18px;
  height: 100%;
  width: 100%;
  display: flex;
  flex-direction: column;
  box-shadow:
    inset 0 1px 0 var(--border-rim),
    0 8px 32px rgba(0,0,0,0.85);
  overflow: hidden;
}

/* Top shimmer overlay */
.liquid-glass-container::before {
  content: '';
  position: absolute;
  inset: 0;
  border-radius: inherit;
  background: linear-gradient(120deg, rgba(255,255,255,0.055) 0%, transparent 55%);
  pointer-events: none;
  z-index: 0;
}

.liquid-glass-container > * {
  position: relative;
  z-index: 1;
}

/* ── Animations ── */
@keyframes fadeIn {
  from { opacity: 0; transform: translateY(-4px); }
  to   { opacity: 1; transform: translateY(0); }
}

@keyframes slideUp {
  from { opacity: 0; transform: translateY(10px); }
  to   { opacity: 1; transform: translateY(0); }
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to   { transform: rotate(360deg); }
}

.animate-fade-in  { animation: fadeIn 0.3s ease-out forwards; }
.animate-slide-up { animation: slideUp 0.35s ease-out forwards; }
.animate-spin     { animation: spin 0.8s linear infinite; }

/* ── Scrollbar ── */
::-webkit-scrollbar       { width: 3px; }
::-webkit-scrollbar-track { background: transparent; }
::-webkit-scrollbar-thumb {
  background: rgba(255,255,255,0.12);
  border-radius: 10px;
}
::-webkit-scrollbar-thumb:hover { background: rgba(255,255,255,0.22); }
```

**Step 2: Verify no syntax errors**

Save the file and check the dev server for any compilation errors.

**Step 3: Commit**

```bash
git add src/index.css
git commit -m "feat: rebuild CSS with Apple liquid glass dark system"
```

---

### Task 4: Update App.tsx — header, logo, and typography

**Files:**
- Modify: `src/App.tsx`

**Step 1: Replace the header section**

Find this block in `App.tsx` (lines 94–107):
```tsx
<header style={{ height: '56px', padding: '0 16px', ... }}>
  <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
    <div style={{ width: '24px', height: '24px', borderRadius: '6px', background: 'linear-gradient(...)' }} />
    <span style={{ fontSize: '15px', fontWeight: 600 }}>OpenTokenMonitor</span>
  </div>
  ...
</header>
```

Replace with:
```tsx
<header style={{
  height: '44px',
  padding: '0 14px',
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'space-between',
  borderBottom: '1px solid var(--border-subtle)',
  flexShrink: 0,
}}>
  <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
    <img
      src="/open_token_monitor_icon.png"
      width={20}
      height={20}
      style={{ borderRadius: '5px' }}
      alt="OpenTokenMonitor"
    />
    <span style={{ fontSize: '13px', fontWeight: 700, letterSpacing: '0.01em' }}>
      OpenTokenMonitor
    </span>
  </div>
  <div style={{ display: 'flex', gap: '6px' }}>
    <button
      onClick={() => refreshData(config)}
      style={{
        background: 'var(--bg-card)',
        border: '1px solid var(--border-subtle)',
        color: 'var(--text-secondary)',
        width: '26px', height: '26px',
        borderRadius: '6px',
        cursor: 'pointer',
        display: 'flex', alignItems: 'center', justifyContent: 'center',
      }}
    >
      <RefreshCw size={13} className={loading ? 'animate-spin' : ''} />
    </button>
    <button
      onClick={() => setView('settings')}
      style={{
        background: 'var(--bg-card)',
        border: '1px solid var(--border-subtle)',
        color: 'var(--text-secondary)',
        width: '26px', height: '26px',
        borderRadius: '6px',
        cursor: 'pointer',
        display: 'flex', alignItems: 'center', justifyContent: 'center',
      }}
    >
      <SettingsIcon size={13} />
    </button>
  </div>
</header>
```

**Step 2: Update the summary bar**

Find this block (lines 109–112):
```tsx
<div style={{ display: 'grid', gridTemplateColumns: 'repeat(2, 1fr)', height: '54px', ... }}>
```

Replace with:
```tsx
<div style={{
  display: 'grid',
  gridTemplateColumns: 'repeat(2, 1fr)',
  height: '44px',
  borderBottom: '1px solid var(--border-subtle)',
  background: 'rgba(255,255,255,0.02)',
  flexShrink: 0,
}}>
  <SummaryCell label="TOTAL COST" value={`$${totalCost.toFixed(2)}`} />
  <SummaryCell label="TOKENS" value={formatTokens(totalTokens)} borderLeft />
</div>
```

**Step 3: Update the tab bar**

Find this block (lines 114–118):
```tsx
<div style={{ display: 'flex', padding: '12px 16px', gap: '8px', overflowX: 'auto' }}>
```

Replace with:
```tsx
<div style={{
  display: 'flex',
  padding: '8px 12px',
  gap: '6px',
  borderBottom: '1px solid var(--border-subtle)',
  background: 'rgba(0,0,0,0.15)',
  flexShrink: 0,
}}>
```

**Step 4: Update the main content area**

Find this block (line 120):
```tsx
<main style={{ flex: 1, overflowY: 'auto', padding: '0 16px 16px 16px' }}>
```

Replace with:
```tsx
<main style={{ flex: 1, overflowY: 'auto', padding: '8px 12px 12px 12px', minHeight: 0 }}>
```

**Step 5: Commit**

```bash
git add src/App.tsx
git commit -m "feat: update header with real logo and compact layout"
```

---

### Task 5: Fix sub-components — SummaryCell, TabButton, ProviderCard, CliActivityItem

**Files:**
- Modify: `src/App.tsx` (bottom section with inline components)

**Step 1: Replace SummaryCell**

Find and replace:
```tsx
const SummaryCell = ({ label, value, borderLeft }: any) => (
  <div style={{ display: 'flex', flexDirection: 'column', justifyContent: 'center', alignItems: 'center', borderLeft: borderLeft ? '1px solid var(--border-subtle)' : 'none' }}>
    <span style={{ fontSize: '9px', color: 'var(--text-secondary)', fontWeight: 600, fontFamily: 'var(--font-mono)' }}>{label}</span>
    <span style={{ fontSize: '15px', fontWeight: 700, fontFamily: 'var(--font-mono)' }}>{value}</span>
  </div>
);
```

With:
```tsx
const SummaryCell = ({ label, value, borderLeft }: any) => (
  <div style={{
    display: 'flex', flexDirection: 'column',
    justifyContent: 'center', alignItems: 'center',
    borderLeft: borderLeft ? '1px solid var(--border-subtle)' : 'none',
  }}>
    <span style={{ fontSize: '10px', color: 'var(--text-muted)', fontWeight: 600, fontFamily: 'var(--font-mono)', letterSpacing: '0.06em' }}>{label}</span>
    <span style={{ fontSize: '15px', fontWeight: 700, fontFamily: 'var(--font-mono)', color: 'var(--text-primary)' }}>{value}</span>
  </div>
);
```

**Step 2: Replace TabButton**

Find and replace:
```tsx
const TabButton = ({ active, onClick, icon, label }: any) => (
  <button onClick={onClick} style={{ display: 'flex', alignItems: 'center', gap: '6px', padding: '6px 12px', borderRadius: '8px', border: 'none', background: active ? 'var(--bg-card-hover)' : 'transparent', color: active ? 'var(--text-primary)' : 'var(--text-secondary)', fontSize: '11px', fontWeight: 600, cursor: 'pointer', whiteSpace: 'nowrap' }}>
    {icon} {label}
  </button>
);
```

With:
```tsx
const TabButton = ({ active, onClick, icon, label }: any) => (
  <button onClick={onClick} style={{
    display: 'flex', alignItems: 'center', gap: '5px',
    padding: '5px 10px',
    borderRadius: '7px',
    border: 'none',
    background: active ? 'rgba(255,255,255,0.10)' : 'transparent',
    color: active ? 'var(--text-primary)' : 'var(--text-secondary)',
    fontSize: '12px', fontWeight: 600,
    cursor: 'pointer', whiteSpace: 'nowrap',
    transition: 'background 0.15s, color 0.15s',
  }}>
    {icon} {label}
  </button>
);
```

**Step 3: Replace ProviderCard with glow**

Find and replace the entire `ProviderCard` component:
```tsx
const ProviderCard = ({ data, delay }: { data: UsageData; delay: string }) => (
  <div className="animate-slide-up" style={{ background: 'var(--bg-card)', borderRadius: '14px', padding: '12px', border: '1px solid var(--border-subtle)', display: 'flex', alignItems: 'center', gap: '12px', animationDelay: delay }}>
```

With:
```tsx
const ProviderCard = ({ data, delay }: { data: UsageData; delay: string }) => (
  <div className="animate-slide-up" style={{
    background: 'var(--bg-card)',
    borderRadius: '12px',
    padding: '10px 12px',
    border: '1px solid var(--border-subtle)',
    display: 'flex', alignItems: 'center', gap: '10px',
    animationDelay: delay,
    boxShadow: `0 0 18px ${data.color}2E`,
  }}>
    <div style={{
      width: '32px', height: '32px', borderRadius: '8px',
      background: data.color,
      display: 'flex', alignItems: 'center', justifyContent: 'center',
      boxShadow: `0 0 14px ${data.color}55`,
      color: 'white', fontWeight: 800, fontSize: '16px', flexShrink: 0,
    }}>
      {data.icon}
    </div>
    <div style={{ flex: 1, minWidth: 0 }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
        <span style={{ fontSize: '13px', fontWeight: 700 }}>{data.displayName}</span>
        <div style={{ width: '5px', height: '5px', borderRadius: '50%', background: data.status === 'ok' ? '#22C55E' : '#EF4444', flexShrink: 0 }} />
      </div>
      <div style={{ height: '3px', width: '100%', background: 'rgba(255,255,255,0.06)', borderRadius: '2px', marginTop: '5px' }}>
        <div style={{ height: '100%', width: '70%', background: data.color, borderRadius: '2px' }} />
      </div>
      <span style={{ fontSize: '11px', color: 'var(--text-secondary)', fontFamily: 'var(--font-mono)', marginTop: '3px', display: 'block' }}>
        {formatTokens(data.totalTokensUsed || 0)} tokens
      </span>
    </div>
    <div style={{ textAlign: 'right', flexShrink: 0 }}>
      <div style={{ fontSize: '14px', fontWeight: 700, fontFamily: 'var(--font-mono)', color: data.color }}>${(data.totalCost || 0).toFixed(2)}</div>
      <div style={{ fontSize: '10px', color: 'var(--text-muted)' }}>usage</div>
    </div>
  </div>
);
```

**Step 4: Replace CliActivityItem**

Find and replace:
```tsx
const CliActivityItem = ({ item }: { item: CliActivity }) => (
  <div className="animate-slide-up" style={{
    background: 'rgba(255, 255, 255, 0.03)',
    borderRadius: '10px',
    padding: '10px',
    borderLeft: `3px solid ${item.provider === 'anthropic' ? 'var(--claude-primary)' : 'var(--openai-primary)'}`
  }}>
    <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: '4px' }}>
      <span style={{ fontSize: '10px', fontWeight: 700, textTransform: 'uppercase', color: 'var(--text-secondary)' }}>
        {item.provider === 'anthropic' ? 'Claude Code' : item.provider}
      </span>
      <span style={{ fontSize: '9px', color: 'var(--text-muted)', display: 'flex', alignItems: 'center', gap: '4px' }}>
        <Clock size={8} /> {new Date(item.timestamp).toLocaleTimeString()}
      </span>
    </div>
    <div style={{ fontSize: '11px', fontFamily: 'var(--font-mono)', color: 'var(--text-primary)', wordBreak: 'break-all' }}>
      <span style={{ color: 'var(--text-secondary)', marginRight: '6px' }}>&gt;</span>
      {item.command}
    </div>
    {item.project && (
      <div style={{ fontSize: '8px', color: 'var(--text-muted)', marginTop: '4px', whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>
        {item.project}
      </div>
    )}
  </div>
);
```

With:
```tsx
const CliActivityItem = ({ item }: { item: CliActivity }) => {
  const accentColor = item.provider === 'anthropic' ? 'var(--claude-primary)' : 'var(--openai-primary)';
  return (
    <div className="animate-slide-up" style={{
      background: 'rgba(255,255,255,0.03)',
      borderRadius: '10px',
      padding: '9px 10px',
      borderLeft: `3px solid ${accentColor}`,
    }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: '3px' }}>
        <span style={{ fontSize: '11px', fontWeight: 700, textTransform: 'uppercase', color: 'var(--text-secondary)', letterSpacing: '0.04em' }}>
          {item.provider === 'anthropic' ? 'Claude Code' : item.provider}
        </span>
        <span style={{ fontSize: '10px', color: 'var(--text-muted)', display: 'flex', alignItems: 'center', gap: '3px' }}>
          <Clock size={9} /> {new Date(item.timestamp).toLocaleTimeString()}
        </span>
      </div>
      <div style={{ fontSize: '12px', fontFamily: 'var(--font-mono)', color: 'var(--text-primary)', wordBreak: 'break-all', lineHeight: 1.4 }}>
        <span style={{ color: accentColor, marginRight: '5px' }}>&gt;</span>
        {item.command}
      </div>
      {item.project && (
        <div style={{ fontSize: '10px', color: 'var(--text-muted)', marginTop: '3px', whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>
          {item.project}
        </div>
      )}
    </div>
  );
};
```

**Step 5: Commit**

```bash
git add src/App.tsx
git commit -m "feat: update all sub-components with glass glows and readable typography"
```

---

### Task 6: Clean up App.css (remove stale Vite template styles)

**Files:**
- Modify: `src/App.css`

**Step 1: Clear the file**

Replace the entire contents of `src/App.css` with an empty file (just a comment):
```css
/* App-specific overrides — global styles live in index.css */
```

This removes the Vite template styles (`.logo.vite`, `.container`, light-mode button styles) that conflict with the dark glass design.

**Step 2: Commit**

```bash
git add src/App.css
git commit -m "chore: clear stale Vite template styles from App.css"
```

---

### Task 7: Launch dev server and verify

**Step 1: Kill any running dev server on port 1420**

```bash
powershell -Command "Get-NetTCPConnection -LocalPort 1420 -ErrorAction SilentlyContinue | ForEach-Object { Stop-Process -Id $_.OwningProcess -Force }"
```

**Step 2: Start dev server**

```bash
npm run tauri dev
```

**Step 3: Verify visually**

- [ ] Window appears immediately on launch (no tray click needed)
- [ ] Window is 360×320px — noticeably smaller than before
- [ ] Header shows the dot-grid logo icon
- [ ] Dark glass with purple-indigo tint visible
- [ ] Top shimmer highlight visible on container
- [ ] All text is readable (no tiny 8–9px labels)
- [ ] CLI activity tab shows feed (or "waiting" placeholder)
- [ ] Stats tab shows provider cards with colored glows
- [ ] Tray icon still works to toggle show/hide

**Step 4: Commit final**

```bash
git add -A
git commit -m "feat: complete UI redesign — liquid glass dark, 360x320, logo, readable text"
```
