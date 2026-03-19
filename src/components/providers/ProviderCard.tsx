import { useState } from 'react';
import { History, RefreshCw } from 'lucide-react';
import CostTrendChart from '@/components/charts/CostTrendChart';
import GlassPanel from '@/components/glass/GlassPanel';
import GlassPill from '@/components/glass/GlassPill';
import ProviderLogo from '@/components/providers/ProviderLogo';
import UsageBar from '@/components/meters/UsageBar';
import { ModelBreakdownEntry, ProviderId, RecentActivityEntry, TrendData, UsageAlert, UsageSnapshot } from '@/types';
import { windowLabel, countdownLabel, windowValueLabel } from '@/utils/usageWindows';

const providerMeta: Record<ProviderId, { name: string; tint: 'claude' | 'codex' | 'gemini'; color: string }> = {
  claude: { name: 'Claude', tint: 'claude', color: '#d97757' },
  codex: { name: 'Codex', tint: 'codex', color: '#10a37f' },
  gemini: { name: 'Gemini', tint: 'gemini', color: '#4285f4' },
};

const severityColor: Record<UsageAlert['severity'], string> = {
  warning: '#f59e0b',
  high: '#f97316',
  critical: '#ef4444',
};

const formatTokens = (value: number) => {
  if (value >= 1_000_000) return `${(value / 1_000_000).toFixed(1)}M`;
  if (value >= 1_000) return `${(value / 1_000).toFixed(1)}K`;
  return String(value);
};

const formatAge = (timestamp: string) => {
  const deltaMs = Date.now() - new Date(timestamp).getTime();
  if (!Number.isFinite(deltaMs) || deltaMs < 0) return 'now';
  const minutes = Math.floor(deltaMs / 60_000);
  if (minutes < 1) return 'now';
  if (minutes < 60) return `${minutes}m`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h`;
  const days = Math.floor(hours / 24);
  return `${days}d`;
};

const fallbackTerminalLabel = (entry?: RecentActivityEntry) => {
  if (!entry) return 'local session';
  if (entry.terminal_label) return entry.terminal_label;
  if (entry.cwd) {
    const parts = entry.cwd.split(/[\\/]/).filter(Boolean);
    return parts[parts.length - 1] ?? entry.cwd;
  }
  return entry.session_id ? `session ${entry.session_id.slice(0, 6)}` : 'local session';
};

type ProviderCardProps = {
  snapshot?: UsageSnapshot;
  trend?: TrendData;
  breakdown?: ModelBreakdownEntry[];
  recentActivity?: RecentActivityEntry[];
  alerts?: UsageAlert[];
  onRefresh: () => void;
};

const ProviderCard = ({ snapshot, trend, breakdown = [], recentActivity = [], alerts = [], onRefresh }: ProviderCardProps) => {
  const [showRecentInputs, setShowRecentInputs] = useState(false);

  if (!snapshot) {
    return (
      <GlassPanel style={{ padding: 14, textAlign: 'center' }}>
        <div className="metric-label">No provider snapshot yet.</div>
      </GlassPanel>
    );
  }

  const meta = providerMeta[snapshot.provider];
  const [primary, secondary] = snapshot.windows;
  const primaryPct = Math.max(0, Math.min(100, primary?.utilization ?? 0));
  const secondaryPct = Math.max(0, Math.min(100, secondary?.utilization ?? 0));
  const costToday = trend?.points[trend.points.length - 1]?.cost_usd ?? 0;

  return (
    <div style={{ display: 'grid', gap: 8 }}>
      {/* Header */}
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
          <ProviderLogo provider={snapshot.provider} size={22} />
          <span style={{ fontWeight: 700, fontSize: 13 }}>{meta.name}</span>
          <span className="metric-label" style={{ fontSize: 9 }}>{snapshot.source}</span>
        </div>
        <div style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
          <GlassPill
            active={showRecentInputs}
            onClick={() => setShowRecentInputs((value) => !value)}
            title="Toggle recent inputs"
            style={{ fontSize: 9, padding: '2px 6px', gap: 3 }}
          >
            <History size={10} /> Recent
          </GlassPill>
          <GlassPill onClick={onRefresh} title="Refresh provider" style={{ fontSize: 9, padding: '2px 6px', gap: 3 }}>
            <RefreshCw size={10} /> Refresh
          </GlassPill>
        </div>
      </div>

      {/* Usage Windows — bars with labels */}
      <GlassPanel tint={meta.tint} style={{ padding: '8px 10px' }}>
        <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
              <span style={{ fontSize: 10, fontWeight: 600 }}>{windowLabel(primary)}</span>
              <span className="metric-label" style={{ fontSize: 8 }}>{countdownLabel(primary)}</span>
            </div>
            <UsageBar pct={primaryPct} />
            {primary?.note && <span className="metric-label" style={{ fontSize: 8, opacity: 0.7, lineHeight: 1.2 }}>{primary.note}</span>}
            <span className="metric-label" style={{ fontSize: 8 }}>{windowValueLabel(primary) ?? ''}</span>
          </div>
          {secondary && (
            <div style={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                <span style={{ fontSize: 10, fontWeight: 600 }}>{windowLabel(secondary)}</span>
                <span className="metric-label" style={{ fontSize: 8 }}>{countdownLabel(secondary)}</span>
              </div>
              <UsageBar pct={secondaryPct} />
              <span className="metric-label" style={{ fontSize: 8 }}>{windowValueLabel(secondary) ?? ''}</span>
            </div>
          )}
        </div>
      </GlassPanel>

      {/* Cost Trend */}
      <GlassPanel style={{ padding: '6px 8px' }}>
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 4 }}>
          <span style={{ fontSize: 11, fontWeight: 700 }}>Cost</span>
          <div style={{ display: 'flex', gap: 4 }}>
            <span className="glass-pill" style={{ fontSize: 9, padding: '1px 5px' }}>Today ${costToday.toFixed(2)}</span>
            <span className="glass-pill" style={{ fontSize: 9, padding: '1px 5px' }}>30d ${trend?.total_cost_usd.toFixed(2) ?? '0.00'}</span>
          </div>
        </div>
        <CostTrendChart points={trend?.points ?? []} color={meta.color} compact />
      </GlassPanel>

      {/* Model Usage */}
      <GlassPanel style={{ padding: '6px 8px' }}>
        <span style={{ fontSize: 11, fontWeight: 700 }}>Models</span>
        {breakdown.length === 0 ? (
          <div className="metric-label" style={{ fontSize: 9, marginTop: 2 }}>No per-model breakdown yet.</div>
        ) : (
          <div style={{ display: 'grid', gap: 3, marginTop: 4 }}>
            {breakdown.map((entry) => (
              <div key={entry.model} style={{ display: 'flex', alignItems: 'center', gap: 6, fontSize: 9 }}>
                <span style={{ fontWeight: 600, flex: 1, minWidth: 0, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{entry.model}</span>
                <span className="metric-label" style={{ fontSize: 8 }}>{formatTokens(entry.input_tokens)}in</span>
                <span className="metric-label" style={{ fontSize: 8 }}>{formatTokens(entry.output_tokens)}out</span>
                <span style={{ fontWeight: 700, fontSize: 9, fontFamily: 'var(--font-mono)' }}>${entry.estimated_cost_usd.toFixed(2)}</span>
              </div>
            ))}
          </div>
        )}
      </GlassPanel>

      {showRecentInputs && (
        <GlassPanel style={{ padding: '6px 8px' }}>
          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: 8 }}>
            <div style={{ display: 'flex', flexDirection: 'column', gap: 1 }}>
              <span style={{ fontSize: 11, fontWeight: 700 }}>Recent Inputs</span>
              <span className="metric-label" style={{ fontSize: 8 }}>Local CLI history for this provider</span>
            </div>
            {breakdown[0] && (
              <span
                className="glass-pill"
                style={{ fontSize: 8, padding: '1px 5px', maxWidth: 138, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}
                title={`${breakdown[0].model} · ${formatTokens(breakdown[0].total_tokens)} tokens`}
              >
                {breakdown[0].model} · {formatTokens(breakdown[0].total_tokens)}
              </span>
            )}
          </div>

          {recentActivity.length === 0 ? (
            <div className="metric-label" style={{ fontSize: 9, marginTop: 6 }}>No recent prompts were detected for this provider yet.</div>
          ) : (
            <div style={{ display: 'grid', gap: 5, marginTop: 6 }}>
              {recentActivity.map((entry, index) => (
                <div
                  key={`${entry.timestamp}-${entry.session_id ?? index}`}
                  style={{
                    display: 'grid',
                    gap: 3,
                    padding: '7px 8px',
                    borderRadius: 10,
                    border: '1px solid rgba(255,255,255,0.08)',
                    background: 'rgba(10, 14, 22, 0.22)',
                  }}
                  title={entry.cwd ?? entry.prompt}
                >
                  <div style={{ display: 'flex', alignItems: 'center', gap: 6, fontSize: 8 }}>
                    <span style={{ fontWeight: 700, color: 'var(--text-primary)' }}>{fallbackTerminalLabel(entry)}</span>
                    <span className="metric-label" style={{ marginLeft: 'auto' }}>{formatAge(entry.timestamp)}</span>
                  </div>
                  <div style={{ fontSize: 10, lineHeight: 1.35, color: 'var(--text-primary)' }}>
                    {entry.prompt}
                  </div>
                </div>
              ))}
            </div>
          )}
        </GlassPanel>
      )}

      {/* Alerts */}
      {alerts.length > 0 && (
        <GlassPanel style={{ padding: '6px 8px' }}>
          <span style={{ fontSize: 11, fontWeight: 700 }}>Alerts</span>
          <div style={{ display: 'grid', gap: 3, marginTop: 4 }}>
            {alerts.map((alert) => (
              <div
                key={`${alert.window_type}-${alert.threshold_percent}`}
                style={{ display: 'flex', alignItems: 'center', gap: 6, fontSize: 9 }}
              >
                <span>{alert.window_type}</span>
                <span style={{ color: severityColor[alert.severity], fontWeight: 700 }}>
                  {alert.severity.toUpperCase()}
                </span>
                <span className="metric-label" style={{ fontSize: 8, marginLeft: 'auto' }}>
                  {alert.utilization.toFixed(0)}% / {alert.threshold_percent}%
                </span>
              </div>
            ))}
          </div>
        </GlassPanel>
      )}
    </div>
  );
};

export default ProviderCard;
