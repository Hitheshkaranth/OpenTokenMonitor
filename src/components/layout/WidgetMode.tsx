import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import GlassPanel from '@/components/glass/GlassPanel';
import ProviderLogo from '@/components/providers/ProviderLogo';
import UsageBar from '@/components/meters/UsageBar';
import { ProviderId, UsageSnapshot } from '@/types';
import { windowLabel, countdownLabel } from '@/utils/usageWindows';
import { isTauriRuntime } from '@/utils/runtime';

const meta: Record<ProviderId, { name: string; tint: 'claude' | 'codex' | 'gemini' }> = {
  claude: { name: 'Claude', tint: 'claude' },
  codex: { name: 'Codex', tint: 'codex' },
  gemini: { name: 'Gemini', tint: 'gemini' },
};

type WidgetModeProps = {
  snapshots: Record<ProviderId, UsageSnapshot | undefined>;
  onExpand: () => void;
};

const WidgetMode = ({ snapshots, onExpand }: WidgetModeProps) => {
  const providers: ProviderId[] = ['claude', 'codex', 'gemini'];

  return (
    <div style={{ height: '100%', display: 'flex', flexDirection: 'column', padding: '6px 8px', gap: 5 }}>
      {/* Header — matches main NavBar style */}
      <div className="nav-header" data-tauri-drag-region style={{ padding: '2px 2px 0' }}>
        <div className="nav-brand" data-tauri-drag-region>
          <img src="/open_token_monitor_icon.png" alt="OTM" className="nav-logo" />
          <span className="nav-title">OpenToken Monitor</span>
        </div>
        <div className="nav-controls">
          <button
            className="glass-pill"
            onClick={onExpand}
            style={{ fontSize: 8, padding: '2px 7px', cursor: 'pointer' }}
          >
            Expand
          </button>
          <button type="button" aria-label="Minimize" className="window-btn"
            onClick={() => { if (isTauriRuntime()) getCurrentWindow().minimize(); }}
            style={{ background: '#febc2e' }}
          />
          <button type="button" aria-label="Close" className="window-btn"
            onClick={() => { if (isTauriRuntime()) invoke('quit_app'); }}
            style={{ background: '#ff5f57' }}
          />
        </div>
      </div>

      {/* Provider cards */}
      {providers.map((id) => {
        const snapshot = snapshots[id];
        const primary = snapshot?.windows[0];
        const secondary = snapshot?.windows[1];
        const pPct = Math.max(0, Math.min(100, primary?.utilization ?? 0));
        const sPct = Math.max(0, Math.min(100, secondary?.utilization ?? 0));
        const m = meta[id];

        return (
          <GlassPanel
            key={id}
            tint={m.tint}
            className={`widget-strip accent-${id}`}
            style={{ flex: 1, padding: '6px 10px', display: 'flex', flexDirection: 'column', justifyContent: 'center', gap: 4, minHeight: 0 }}
          >
            {/* Provider header */}
            <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
              <ProviderLogo provider={id} size={18} />
              <span style={{ fontWeight: 700, fontSize: 11, color: 'var(--text-primary)' }}>{m.name}</span>
              <span className="metric-label" style={{ fontSize: 8, marginLeft: 'auto' }}>
                {countdownLabel(primary)}
              </span>
            </div>

            {/* Usage bars */}
            <div style={{ display: 'flex', flexDirection: 'column', gap: 3 }}>
              <UsageBar pct={pPct} label={windowLabel(primary)?.split(' ')[0]} />
              {secondary && <UsageBar pct={sPct} label={windowLabel(secondary)?.split(' ')[0]} />}
            </div>
          </GlassPanel>
        );
      })}
    </div>
  );
};

export default WidgetMode;
