import ProviderLogo from '@/components/providers/ProviderLogo';
import { ProviderId, ProviderStatus, UsageAlert, UsageSnapshot } from '@/types';
import { getProviderAccessState } from '@/utils/providerAccess';

const providers: ProviderId[] = ['claude', 'codex', 'gemini'];

type DiagnosticsPanelProps = {
  statuses: Record<ProviderId, ProviderStatus | undefined>;
  snapshots: Record<ProviderId, UsageSnapshot | undefined>;
  alerts: Record<ProviderId, UsageAlert[]>;
  globalError?: string;
};

const DiagnosticsPanel = ({ statuses, snapshots, alerts, globalError }: DiagnosticsPanelProps) => (
  <div className="stg-section stg-diag">
    <span className="stg-section-title" style={{ marginBottom: 2 }}>Diagnostics</span>
    {globalError && (
      <div className="stg-diag-error">Error: {globalError}</div>
    )}
    <div className="stg-diag-list">
      {providers.map((provider) => {
        const status = statuses[provider];
        const snapshot = snapshots[provider];
        const access = getProviderAccessState(status, snapshot);
        return (
          <div key={provider} className="stg-diag-row">
            <ProviderLogo provider={provider} size={12} />
            <span className="stg-diag-name">{provider}</span>
            <span className="stg-diag-status" style={{ color: access.color }}>{access.label}</span>
            <span className="stg-diag-info">
              {snapshot ? `${snapshot.source} @ ${new Date(snapshot.fetched_at).toLocaleTimeString()}` : 'no data'}
            </span>
            <span className="stg-diag-alerts">{alerts[provider]?.length || 0} alerts</span>
          </div>
        );
      })}
    </div>
  </div>
);

export default DiagnosticsPanel;
