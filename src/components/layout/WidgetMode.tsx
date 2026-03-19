import { RefreshCw } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import GlassPanel from '@/components/glass/GlassPanel';
import WidgetGauge, { arcColor } from '@/components/meters/WidgetGauge';
import ResetCountdown from '@/components/meters/ResetCountdown';
import { ProviderId, ProviderStatus, UsageSnapshot } from '@/types';
import { isTauriRuntime } from '@/utils/runtime';

const meta: Record<ProviderId, { name: string; tint: 'claude' | 'codex' | 'gemini' }> = {
  claude: { name: 'Claude', tint: 'claude' },
  codex: { name: 'Codex', tint: 'codex' },
  gemini: { name: 'Gemini', tint: 'gemini' },
};

type WidgetModeProps = {
  snapshots: Record<ProviderId, UsageSnapshot | undefined>;
  statuses: Partial<Record<ProviderId, ProviderStatus>>;
  onExpand: () => void;
  onRefresh: () => void;
  refreshBusy: boolean;
};

const WidgetMode = ({ snapshots, statuses, onExpand, onRefresh, refreshBusy }: WidgetModeProps) => {
  const providers: ProviderId[] = ['claude', 'codex', 'gemini'];

  return (
    <div style={{ height: '100%', display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
      {/* ── Header: identical to main NavBar ── */}
      <div className="nav-bar" data-tauri-drag-region style={{ flexShrink: 0 }}>
        <div className="nav-header" data-tauri-drag-region>
          <div className="nav-brand" data-tauri-drag-region>
            <img src="/open_token_monitor_icon.png" alt="OTM" className="nav-logo" />
            <span className="nav-title">OpenToken Monitor</span>
          </div>
          <div className="nav-controls">
            <button
              className="glass-pill compact-action-btn"
              onClick={onRefresh}
              disabled={refreshBusy}
              title="Refresh all (Ctrl+R)"
              style={{ width: 26, height: 26, minWidth: 26 }}
            >
              <RefreshCw size={12} className={refreshBusy ? 'spin-icon' : ''} />
            </button>
            <button
              onClick={onExpand}
              title="Expand to full view"
              style={{
                display: 'inline-flex',
                alignItems: 'center',
                justifyContent: 'center',
                gap: 3,
                height: 22,
                padding: '0 10px',
                fontSize: 10,
                fontWeight: 600,
                letterSpacing: '.02em',
                color: 'var(--text-primary)',
                background: 'rgba(255,255,255,0.12)',
                border: '1px solid rgba(255,255,255,0.18)',
                borderRadius: 6,
                cursor: 'pointer',
                backdropFilter: 'blur(8px)',
                WebkitBackdropFilter: 'blur(8px)',
                transition: 'background .15s, border-color .15s',
              }}
              onMouseEnter={(e) => {
                e.currentTarget.style.background = 'rgba(255,255,255,0.22)';
                e.currentTarget.style.borderColor = 'rgba(255,255,255,0.3)';
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.background = 'rgba(255,255,255,0.12)';
                e.currentTarget.style.borderColor = 'rgba(255,255,255,0.18)';
              }}
            >
              <svg width="10" height="10" viewBox="0 0 10 10" fill="none" style={{ flexShrink: 0 }}>
                <path d="M1 9L9 1M9 1H3.5M9 1V6.5" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" strokeLinejoin="round"/>
              </svg>
              Expand
            </button>
            <button type="button" aria-label="Minimize" title="Minimize" className="window-btn"
              onClick={() => { if (isTauriRuntime()) getCurrentWindow().minimize(); }}
              style={{ background: '#febc2e' }}
            />
            <button type="button" aria-label="Close" title="Close" className="window-btn"
              onClick={() => { if (isTauriRuntime()) invoke('quit_app'); }}
              style={{ background: '#ff5f57' }}
            />
          </div>
        </div>
      </div>

      {/* ── 3 cards in ONE horizontal row ── */}
      <div style={{ display: 'flex', flexDirection: 'row', flex: 1, minHeight: 0, padding: '5px 8px 8px', gap: 6 }}>
        {providers.map((id) => {
          const snapshot = snapshots[id];
          const primary = snapshot?.windows[0];
          const secondary = snapshot?.windows[1];
          const pPct = Math.max(0, Math.min(100, primary?.utilization ?? 0));
          const sPct = secondary ? Math.max(0, Math.min(100, secondary.utilization ?? 0)) : undefined;
          const m = meta[id];


          return (
            <GlassPanel
              key={id}
              tint={m.tint}
              style={{
                flex: 1,
                display: 'flex',
                flexDirection: 'column',
                alignItems: 'center',
                justifyContent: 'center',
                padding: '6px 4px',
                gap: 3,
                minWidth: 0,
                minHeight: 0,
                borderRadius: 12,
                overflow: 'hidden',
                position: 'relative',
              }}
            >
              {/* Provider name + health dot */}
              <div style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
                <span style={{
                  fontWeight: 700,
                  fontSize: 13,
                  color: 'var(--text-primary)',
                  whiteSpace: 'nowrap',
                  letterSpacing: '.03em',
                }}>
                  {m.name}
                </span>
                <span className={`nav-tab-dot ${statuses[id]?.health ? `health-${statuses[id]!.health}` : 'health-unknown'}`} />
              </div>

              {/* Apple Activity Ring with logo */}
              <WidgetGauge provider={id} primaryPct={pPct} secondaryPct={sPct} />

              {/* Usage stats below ring */}
              <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 0 }}>
                <span style={{ fontSize: 10, fontWeight: 700, fontFamily: 'var(--font-mono)', color: arcColor(pPct) }}>
                  <span style={{ opacity: 0.55 }}><ResetCountdown resetsAt={primary?.resets_at} className="" /></span> {pPct.toFixed(0)}%
                </span>
                {sPct != null && (
                  <span style={{ fontSize: 10, fontWeight: 700, fontFamily: 'var(--font-mono)', color: arcColor(sPct) }}>
                    <span style={{ opacity: 0.55 }}><ResetCountdown resetsAt={secondary?.resets_at} className="" /></span> {sPct.toFixed(0)}%
                  </span>
                )}
              </div>
            </GlassPanel>
          );
        })}
      </div>
    </div>
  );
};

export default WidgetMode;
