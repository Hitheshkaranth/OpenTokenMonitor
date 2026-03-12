import { RefreshCw } from 'lucide-react';
import WindowMeter from '@/components/meters/WindowMeter';
import CostTrendChart from '@/components/charts/CostTrendChart';
import GlassPill from '@/components/glass/GlassPill';
import ProviderLogo from '@/components/providers/ProviderLogo';
import { ModelBreakdownEntry, ProviderId, TrendData, UsageAlert, UsageSnapshot } from '@/types';

const providerMeta: Record<ProviderId, { name: string; tint: 'claude' | 'codex' | 'gemini'; color: string }> = {
  claude: { name: 'Claude', tint: 'claude', color: '#d97757' },
  codex: { name: 'Codex', tint: 'codex', color: '#10a37f' },
  gemini: { name: 'Gemini', tint: 'gemini', color: '#4285f4' },
};

type ProviderCardProps = {
  snapshot?: UsageSnapshot;
  trend?: TrendData;
  breakdown?: ModelBreakdownEntry[];
  alerts?: UsageAlert[];
  onRefresh: () => void;
};

const formatTokens = (value: number) => {
  if (value >= 1_000_000) return `${(value / 1_000_000).toFixed(1)}M`;
  if (value >= 1_000) return `${(value / 1_000).toFixed(1)}K`;
  return String(value);
};

const severityColor: Record<UsageAlert['severity'], string> = {
  warning: '#f59e0b',
  high: '#f97316',
  critical: '#ef4444',
};

const ProviderCard = ({ snapshot, trend, breakdown = [], alerts = [], onRefresh }: ProviderCardProps) => {
  if (!snapshot) {
    return <div className="glass-panel" style={{ padding: 14 }}>No provider snapshot yet.</div>;
  }

  const meta = providerMeta[snapshot.provider];
  const [primary, secondary] = snapshot.windows;
  const costToday = trend?.points[trend.points.length - 1]?.cost_usd ?? 0;

  return (
    <div className={`glass-panel glass-${meta.tint} hover-lift`} style={{ padding: 14, display: 'grid', gap: 12 }}>
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
        <div>
          <div className="provider-name" style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
            <ProviderLogo provider={snapshot.provider} size={16} />
            <span>{meta.name}</span>
          </div>
          <div className="metric-label">Source: {snapshot.source}</div>
        </div>
        <GlassPill onClick={onRefresh} title="Refresh provider">
          <RefreshCw size={14} /> Refresh
        </GlassPill>
      </div>

      <WindowMeter primary={primary} secondary={secondary} providerTint={meta.tint} />

      <div style={{ display: 'flex', gap: 8, flexWrap: 'wrap' }}>
        <span className="glass-pill">Today ${costToday.toFixed(2)}</span>
        <span className="glass-pill">30d ${trend?.total_cost_usd.toFixed(2) ?? '0.00'}</span>
        <span className="glass-pill">Provenance {snapshot.provenance ?? 'derived_local'}</span>
        <span className="glass-pill">Updated {new Date(snapshot.fetched_at).toLocaleTimeString()}</span>
      </div>

      <CostTrendChart points={trend?.points ?? []} color={meta.color} />

      <div className="glass-panel" style={{ padding: 10, display: 'grid', gap: 8 }}>
        <div className="provider-name" style={{ fontSize: 12 }}>Model Mix</div>
        {breakdown.length === 0 ? (
          <div className="metric-label">No per-model breakdown available yet.</div>
        ) : (
          breakdown.slice(0, 4).map((entry) => (
            <div key={entry.model} className="glass-pill" style={{ justifyContent: 'space-between', gap: 8 }}>
              <span>{entry.model}</span>
              <span className="metric-label">{formatTokens(entry.total_tokens)} tokens</span>
              <span className="metric-label">${entry.estimated_cost_usd.toFixed(2)}</span>
            </div>
          ))
        )}
      </div>

      <div className="glass-panel" style={{ padding: 10, display: 'grid', gap: 8 }}>
        <div className="provider-name" style={{ fontSize: 12 }}>Alerts</div>
        {alerts.length === 0 ? (
          <div className="metric-label">No active threshold alerts.</div>
        ) : (
          alerts.map((alert) => (
            <div key={`${alert.window_type}-${alert.threshold_percent}`} className="glass-pill" style={{ justifyContent: 'space-between', gap: 8 }}>
              <span>{alert.window_type}</span>
              <span className="metric-label" style={{ color: severityColor[alert.severity] }}>
                {alert.utilization.toFixed(0)}% / {alert.threshold_percent}%
              </span>
            </div>
          ))
        )}
      </div>
    </div>
  );
};

export default ProviderCard;
