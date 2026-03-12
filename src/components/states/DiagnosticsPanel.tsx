import { ProviderId, ProviderStatus, UsageAlert, UsageSnapshot } from '@/types';

const providers: ProviderId[] = ['claude', 'codex', 'gemini'];

const healthColor = (health?: ProviderStatus['health']) => {
  if (health === 'active') return '#34d399';
  if (health === 'error') return '#f87171';
  return '#fbbf24';
};

type DiagnosticsPanelProps = {
  statuses: Record<ProviderId, ProviderStatus | undefined>;
  snapshots: Record<ProviderId, UsageSnapshot | undefined>;
  alerts: Record<ProviderId, UsageAlert[]>;
  globalError?: string;
  demoMode: boolean;
};

const DiagnosticsPanel = ({ statuses, snapshots, alerts, globalError, demoMode }: DiagnosticsPanelProps) => (
  <div className="glass-panel" style={{ padding: 10, display: 'grid', gap: 8 }}>
    <div className="provider-name">Diagnostics</div>
    <div className="metric-label">Demo mode: {demoMode ? 'ON' : 'OFF'}</div>
    {globalError ? (
      <div className="metric-label" style={{ color: '#f87171' }}>
        Last refresh error: {globalError}
      </div>
    ) : (
      <div className="metric-label">Last refresh error: none</div>
    )}
    {providers.map((provider) => {
      const status = statuses[provider];
      const snapshot = snapshots[provider];
      return (
        <div key={provider} className="glass-pill" style={{ justifyContent: 'space-between', gap: 10 }}>
          <span style={{ textTransform: 'capitalize' }}>{provider}</span>
          <span style={{ color: healthColor(status?.health) }}>{status?.health ?? 'unknown'}</span>
          <span className="metric-label" style={{ minWidth: 140, textAlign: 'right' }}>
            {snapshot ? `${snapshot.source} @ ${new Date(snapshot.fetched_at).toLocaleTimeString()}` : 'no snapshot'}
          </span>
          <span className="metric-label" style={{ minWidth: 68, textAlign: 'right' }}>
            {alerts[provider]?.length ? `${alerts[provider].length} alerts` : '0 alerts'}
          </span>
        </div>
      );
    })}
  </div>
);

export default DiagnosticsPanel;
