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
};

const DiagnosticsPanel = ({ statuses, snapshots, alerts, globalError }: DiagnosticsPanelProps) => (
  <div className="glass-panel" style={{ padding: '6px 8px', display: 'grid', gap: 4 }}>
    <div style={{ fontSize: 11, fontWeight: 700, color: 'var(--text-secondary)' }}>Diagnostics</div>
    {globalError && (
      <div className="metric-label" style={{ color: '#f87171', fontSize: 9 }}>
        Error: {globalError}
      </div>
    )}
    {providers.map((provider) => {
      const status = statuses[provider];
      const snapshot = snapshots[provider];
      return (
        <div key={provider} style={{ display: 'flex', alignItems: 'center', gap: 6, fontSize: 9 }}>
          <span style={{ textTransform: 'capitalize', fontWeight: 600, width: 44 }}>{provider}</span>
          <span style={{ color: healthColor(status?.health), fontWeight: 600 }}>{status?.health ?? '?'}</span>
          <span className="metric-label" style={{ fontSize: 8, flex: 1, textAlign: 'right', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
            {snapshot ? `${snapshot.source} @ ${new Date(snapshot.fetched_at).toLocaleTimeString()}` : 'no data'}
          </span>
          <span className="metric-label" style={{ fontSize: 8 }}>
            {alerts[provider]?.length || 0} alerts
          </span>
        </div>
      );
    })}
  </div>
);

export default DiagnosticsPanel;
