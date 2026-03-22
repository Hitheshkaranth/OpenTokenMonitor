import { useEffect, useState } from 'react';
import { Gauge, History, RefreshCw } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import GlassPanel from '@/components/glass/GlassPanel';
import WidgetActivityView from '@/components/layout/WidgetActivityView';
import WidgetGauge, { arcColor } from '@/components/meters/WidgetGauge';
import ResetCountdown from '@/components/meters/ResetCountdown';
import { ModelBreakdownEntry, ProviderId, ProviderStatus, RecentActivityEntry, UsageSnapshot, WindowType } from '@/types';
import { getProviderAccessState, providerAccessDotClass } from '@/utils/providerAccess';
import { isTauriRuntime } from '@/utils/runtime';
import { displayWindows } from '@/utils/usageWindows';

const meta: Record<ProviderId, { name: string; tint: 'claude' | 'codex' | 'gemini' }> = {
  claude: { name: 'Claude', tint: 'claude' },
  codex: { name: 'Codex', tint: 'codex' },
  gemini: { name: 'Gemini', tint: 'gemini' },
};

const providers: ProviderId[] = ['claude', 'codex', 'gemini'];

const widgetWindowLabel = (windowType?: WindowType) => {
  switch (windowType) {
    case 'five_hour':
      return '5H';
    case 'seven_day':
      return '7D';
    case 'daily':
      return 'DAY';
    case 'monthly':
      return 'MO';
    case 'weekly':
      return 'WK';
    case 'session':
      return 'SES';
    default:
      return 'WIN';
  }
};

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
          {/* Left: logo + title + refresh */}
          <div className="nav-header-left" data-tauri-drag-region>
            <img src="/open_token_monitor_icon.png" alt="OTM" className="nav-logo" />
            <span className="nav-title" data-tauri-drag-region>OpenToken Monitor</span>
            <button
              className="nav-action-btn"
              onClick={onRefresh}
              disabled={refreshBusy}
              title="Refresh all (Ctrl+R)"
            >
              <RefreshCw size={11} className={refreshBusy ? 'spin-icon' : ''} />
            </button>
          </div>

          {/* Center: screen toggle pills */}
          <nav className="nav-pill-bar" data-tauri-drag-region>
            <button
              className={`nav-pill ${screen === 'usage' ? 'nav-pill-active' : ''}`}
              onClick={() => setScreen('usage')}
              title="Usage rings"
            >
              <Gauge size={12} />
            </button>
            <button
              className={`nav-pill ${screen === 'activity' ? 'nav-pill-active' : ''}`}
              onClick={() => setScreen('activity')}
              title="Recent conversations"
            >
              <History size={12} />
            </button>
          </nav>

          {/* Right: expand + window controls */}
          <div className="nav-header-right">
            <button
              className="nav-action-btn"
              onClick={onExpand}
              title="Expand to full view"
            >
              <svg width="10" height="10" viewBox="0 0 10 10" fill="none">
                <path d="M1 9L9 1M9 1H3.5M9 1V6.5" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" strokeLinejoin="round" />
              </svg>
            </button>
            <div className="nav-traffic-lights">
              <button type="button" aria-label="Minimize" title="Minimize" className="window-btn window-btn-minimize"
                onClick={() => { if (isTauriRuntime()) getCurrentWindow().minimize(); }}
              />
              <button type="button" aria-label="Close" title="Close" className="window-btn window-btn-close"
                onClick={() => { if (isTauriRuntime()) invoke('quit_app'); }}
              />
            </div>
          </div>
        </div>
      </div>

      {screen === 'usage' ? (
        <div className="widget-usage-grid">
          {providers.map((id) => {
            const snapshot = snapshots[id];
            const access = getProviderAccessState(statuses[id], snapshot);
            const [primary, secondary] = displayWindows(snapshot);
            const primaryPct = Math.max(0, Math.min(100, primary?.utilization ?? 0));
            const secondaryPct = secondary ? Math.max(0, Math.min(100, secondary.utilization ?? 0)) : undefined;
            const emptyStateLabel = access.health === 'error' ? 'Unavailable' : 'Awaiting';

            return (
              <GlassPanel
                key={id}
                className="widget-provider-card"
                tint={meta[id].tint}
                onClick={() => openActivity(id)}
                title={access.detail}
              >
                <div className="widget-provider-header">
                  <div className="widget-provider-title-row">
                    <span className="widget-provider-title">{meta[id].name}</span>
                    <span className={`nav-tab-dot ${providerAccessDotClass(access.health)}`} />
                  </div>
                </div>

                <div className="widget-provider-gauge-wrap">
                  <WidgetGauge provider={id} primaryPct={primaryPct} secondaryPct={secondaryPct} />
                </div>

                <div className="widget-provider-metrics">
                  {snapshot ? (
                    <>
                      <div className="widget-provider-complication">
                        <div className="widget-provider-complication-head">
                          <span className="widget-provider-window-label">
                            {widgetWindowLabel(primary?.window_type)}
                          </span>
                          <span className="widget-provider-value" style={{ color: arcColor(primaryPct) }}>
                            {primaryPct.toFixed(0)}%
                          </span>
                        </div>
                        <ResetCountdown resetsAt={primary?.resets_at} className="widget-provider-reset" />
                      </div>
                      {secondaryPct != null && (
                        <div className="widget-provider-complication">
                          <div className="widget-provider-complication-head">
                            <span className="widget-provider-window-label">
                              {widgetWindowLabel(secondary?.window_type)}
                            </span>
                            <span className="widget-provider-value" style={{ color: arcColor(secondaryPct) }}>
                              {secondaryPct.toFixed(0)}%
                            </span>
                          </div>
                          <ResetCountdown resetsAt={secondary?.resets_at} className="widget-provider-reset" />
                        </div>
                      )}
                    </>
                  ) : (
                    <span className="widget-provider-empty" title={access.detail}>
                      {emptyStateLabel}
                    </span>
                  )}
                  <div className="widget-provider-status-row">
                    <span
                      className="widget-provider-status-badge"
                      style={{ color: access.color, borderColor: access.color }}
                    >
                      {access.label}
                    </span>
                  </div>
                </div>
              </GlassPanel>
            );
          })}
        </div>
      ) : (
        <WidgetActivityView
          provider={activityProvider}
          statuses={statuses}
          snapshots={snapshots}
          modelBreakdowns={modelBreakdowns}
          recentActivity={recentActivity}
          onSelectProvider={setActivityProvider}
        />
      )}
    </div>
  );
};

export default WidgetMode;
