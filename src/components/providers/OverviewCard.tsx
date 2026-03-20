import GlassPanel from '@/components/glass/GlassPanel';
import ProviderLogo from '@/components/providers/ProviderLogo';
import Sparkline from '@/components/charts/Sparkline';
import UsageBar from '@/components/meters/UsageBar';
import {
  ModelBreakdownEntry,
  ProviderId,
  ProviderStatus,
  TrendData,
  UsageAlert,
  UsageSnapshot,
} from '@/types';
import { getProviderAccessState, providerAccessDotClass } from '@/utils/providerAccess';
import { countdownLabel } from '@/utils/usageWindows';

const providerMeta: Record<ProviderId, { label: string; tint: 'claude' | 'codex' | 'gemini'; color: string }> = {
  claude: { label: 'Claude', tint: 'claude', color: '#d97757' },
  codex: { label: 'Codex', tint: 'codex', color: '#10a37f' },
  gemini: { label: 'Gemini', tint: 'gemini', color: '#4285f4' },
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

type OverviewCardProps = {
  provider: ProviderId;
  snapshot?: UsageSnapshot;
  trend?: TrendData;
  breakdown?: ModelBreakdownEntry[];
  alerts?: UsageAlert[];
  status?: ProviderStatus;
  onClick: () => void;
};

const OverviewCard = ({ provider, snapshot, trend, breakdown = [], alerts = [], status, onClick }: OverviewCardProps) => {
  const meta = providerMeta[provider];
  const primary = snapshot?.windows[0];
  const secondary = snapshot?.windows[1];
  const primaryPct = Math.max(0, Math.min(100, primary?.utilization ?? 0));
  const secondaryPct = Math.max(0, Math.min(100, secondary?.utilization ?? 0));
  const costToday = trend?.points[trend.points.length - 1]?.cost_usd ?? 0;
  const access = getProviderAccessState(status, snapshot);
  const healthClass = providerAccessDotClass(access.health);

  return (
    <GlassPanel
      tint={meta.tint}
      className={`hover-lift overview-card accent-${provider}`}
      style={{ padding: '8px 10px', cursor: 'pointer', flex: 1, minHeight: 0, display: 'flex', flexDirection: 'column', gap: 5 }}
      onClick={onClick}
    >
      {/* Row 1: Provider identity + cost + sparkline */}
      <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
        <ProviderLogo provider={provider} size={22} />
        <div style={{ display: 'flex', flexDirection: 'column', gap: 0, minWidth: 0 }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 5 }}>
            <span style={{ fontWeight: 700, fontSize: 12, whiteSpace: 'nowrap' }}>{meta.label}</span>
            <span className={`nav-tab-dot ${healthClass}`} />
            {access.health !== 'active' && (
              <span
                className="glass-pill"
                style={{
                  fontSize: 7,
                  padding: '0 5px',
                  color: access.color,
                  borderColor: access.color,
                }}
              >
                {access.label}
              </span>
            )}
          </div>
          <div style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
            <span style={{ fontSize: 9, fontWeight: 600, fontFamily: 'var(--font-mono)', color: 'var(--text-secondary)' }}>
              ${costToday.toFixed(2)}
            </span>
            <span className="metric-label" style={{ fontSize: 8 }}>
              / ${trend?.total_cost_usd.toFixed(2) ?? '0.00'} 30d
            </span>
          </div>
        </div>
        <div style={{ flex: 1, minWidth: 30 }}>
          <Sparkline points={trend?.points ?? []} color={meta.color} height={28} />
        </div>
      </div>

      {/* Row 2: Usage bars with countdown labels */}
      <div style={{ display: 'flex', flexDirection: 'column', gap: 3 }}>
        <UsageBar pct={primaryPct} label={countdownLabel(primary)} />
        {secondary && <UsageBar pct={secondaryPct} label={countdownLabel(secondary)} />}
      </div>

      {access.health !== 'active' && (
        <div
          className="metric-label"
          style={{
            fontSize: 8,
            color: access.color,
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            whiteSpace: 'nowrap',
          }}
          title={access.detail}
        >
          {access.detail}
        </div>
      )}

      {/* Row 3: Model + alerts (compact) */}
      {(breakdown.length > 0 || alerts.length > 0) && (
        <div style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
          {breakdown.length > 0 && (
            <span className="metric-label" style={{ fontSize: 8 }}>
              {breakdown[0].model} · {formatTokens(breakdown[0].total_tokens)}
            </span>
          )}
          {alerts.length > 0 && (
            <div style={{ display: 'flex', gap: 3, marginLeft: 'auto' }}>
              {alerts.slice(0, 2).map((alert) => (
                <span
                  key={`${alert.window_type}-${alert.threshold_percent}`}
                  className="glass-pill"
                  style={{
                    fontSize: 8,
                    padding: '0px 4px',
                    color: severityColor[alert.severity],
                    borderColor: severityColor[alert.severity],
                  }}
                >
                  {alert.severity}
                </span>
              ))}
            </div>
          )}
        </div>
      )}
    </GlassPanel>
  );
};

export default OverviewCard;
