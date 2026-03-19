import { useEffect, useState } from 'react';
import { Gauge, History, RefreshCw } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import GlassPill from '@/components/glass/GlassPill';
import GlassPanel from '@/components/glass/GlassPanel';
import WidgetActivityView from '@/components/layout/WidgetActivityView';
import WidgetGauge, { arcColor } from '@/components/meters/WidgetGauge';
import ResetCountdown from '@/components/meters/ResetCountdown';
import { ModelBreakdownEntry, ProviderId, ProviderStatus, RecentActivityEntry, UsageSnapshot } from '@/types';
import { isTauriRuntime } from '@/utils/runtime';

const meta: Record<ProviderId, { name: string; tint: 'claude' | 'codex' | 'gemini' }> = {
  claude: { name: 'Claude', tint: 'claude' },
  codex: { name: 'Codex', tint: 'codex' },
  gemini: { name: 'Gemini', tint: 'gemini' },
};

const providers: ProviderId[] = ['claude', 'codex', 'gemini'];

type WidgetModeProps = {
  snapshots: Record<ProviderId, UsageSnapshot | undefined>;
  statuses: Partial<Record<ProviderId, ProviderStatus>>;
  modelBreakdowns: Record<ProviderId, ModelBreakdownEntry[]>;
  recentActivity: Record<ProviderId, RecentActivityEntry[]>;
  onExpand: () => void;
  onRefresh: () => void;
  refreshBusy: boolean;
};

type WidgetScreen = 'usage' | 'activity';

const WidgetMode = ({
  snapshots,
  statuses,
  modelBreakdowns,
  recentActivity,
  onExpand,
  onRefresh,
  refreshBusy,
}: WidgetModeProps) => {
  const [screen, setScreen] = useState<WidgetScreen>('usage');
  const [activityProvider, setActivityProvider] = useState<ProviderId>('codex');

  useEffect(() => {
    if (recentActivity[activityProvider]?.length > 0) return;
    const fallback = providers.find((provider) => recentActivity[provider]?.length > 0);
    if (fallback) {
      setActivityProvider(fallback);
    }
  }, [activityProvider, recentActivity]);

  const openActivity = (provider: ProviderId) => {
    setActivityProvider(provider);
    setScreen('activity');
  };

  return (
    <div style={{ height: '100%', display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
      <div className="nav-bar" data-tauri-drag-region style={{ flexShrink: 0 }}>
        <div className="nav-header" data-tauri-drag-region>
          <div className="nav-brand" data-tauri-drag-region>
            <img src="/open_token_monitor_icon.png" alt="OTM" className="nav-logo" />
            <span className="nav-title">OpenToken Monitor</span>
          </div>
          <div className="nav-controls">
            <div style={{ display: 'flex', alignItems: 'center', gap: 4, marginRight: 2 }}>
              <GlassPill
                active={screen === 'usage'}
                onClick={() => setScreen('usage')}
                title="Usage rings"
                style={{
                  display: 'inline-flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  gap: 0,
                  width: 26,
                  height: 26,
                  minWidth: 26,
                  padding: 0,
                  fontSize: 9,
                }}
              >
                <Gauge size={11} />
              </GlassPill>
              <GlassPill
                active={screen === 'activity'}
                onClick={() => setScreen('activity')}
                title="Recent inputs"
                style={{
                  display: 'inline-flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  gap: 0,
                  width: 26,
                  height: 26,
                  minWidth: 26,
                  padding: 0,
                  fontSize: 9,
                }}
              >
                <History size={11} />
              </GlassPill>
            </div>
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
              type="button"
              aria-label="Expand to full view"
              onClick={onExpand}
              title="Expand to full view"
              style={{
                display: 'inline-flex',
                alignItems: 'center',
                justifyContent: 'center',
                width: 24,
                height: 24,
                minWidth: 24,
                padding: 0,
                color: 'var(--text-primary)',
                background: 'rgba(255,255,255,0.12)',
                border: '1px solid rgba(255,255,255,0.18)',
                borderRadius: 7,
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
              <svg width="9" height="9" viewBox="0 0 10 10" fill="none" style={{ flexShrink: 0 }}>
                <path d="M1 9L9 1M9 1H3.5M9 1V6.5" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" strokeLinejoin="round" />
              </svg>
            </button>
            <button
              type="button"
              aria-label="Minimize"
              title="Minimize"
              className="window-btn"
              onClick={() => {
                if (isTauriRuntime()) getCurrentWindow().minimize();
              }}
              style={{ background: '#febc2e' }}
            />
            <button
              type="button"
              aria-label="Close"
              title="Close"
              className="window-btn"
              onClick={() => {
                if (isTauriRuntime()) invoke('quit_app');
              }}
              style={{ background: '#ff5f57' }}
            />
          </div>
        </div>
      </div>

      {screen === 'usage' ? (
        <div style={{ display: 'flex', flexDirection: 'row', flex: 1, minHeight: 0, padding: '5px 8px 8px', gap: 6 }}>
          {providers.map((id) => {
            const snapshot = snapshots[id];
            const primary = snapshot?.windows[0];
            const secondary = snapshot?.windows[1];
            const primaryPct = Math.max(0, Math.min(100, primary?.utilization ?? 0));
            const secondaryPct = secondary ? Math.max(0, Math.min(100, secondary.utilization ?? 0)) : undefined;

            return (
              <GlassPanel
                key={id}
                tint={meta[id].tint}
                onClick={() => openActivity(id)}
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
                  cursor: 'pointer',
                }}
              >
                <div style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
                  <span
                    style={{
                      fontWeight: 700,
                      fontSize: 13,
                      color: 'var(--text-primary)',
                      whiteSpace: 'nowrap',
                      letterSpacing: '.03em',
                    }}
                  >
                    {meta[id].name}
                  </span>
                  <span className={`nav-tab-dot ${statuses[id]?.health ? `health-${statuses[id]!.health}` : 'health-unknown'}`} />
                </div>

                <WidgetGauge provider={id} primaryPct={primaryPct} secondaryPct={secondaryPct} />

                <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 0 }}>
                  <span style={{ fontSize: 10, fontWeight: 700, fontFamily: 'var(--font-mono)', color: arcColor(primaryPct) }}>
                    <span style={{ opacity: 0.55 }}>
                      <ResetCountdown resetsAt={primary?.resets_at} className="" />
                    </span>{' '}
                    {primaryPct.toFixed(0)}%
                  </span>
                  {secondaryPct != null && (
                    <span style={{ fontSize: 10, fontWeight: 700, fontFamily: 'var(--font-mono)', color: arcColor(secondaryPct) }}>
                      <span style={{ opacity: 0.55 }}>
                        <ResetCountdown resetsAt={secondary?.resets_at} className="" />
                      </span>{' '}
                      {secondaryPct.toFixed(0)}%
                    </span>
                  )}
                </div>
              </GlassPanel>
            );
          })}
        </div>
      ) : (
        <WidgetActivityView
          provider={activityProvider}
          statuses={statuses}
          modelBreakdowns={modelBreakdowns}
          recentActivity={recentActivity}
          onSelectProvider={setActivityProvider}
        />
      )}
    </div>
  );
};

export default WidgetMode;
