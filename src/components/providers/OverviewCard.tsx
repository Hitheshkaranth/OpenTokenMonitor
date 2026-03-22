import ProviderLogo from '@/components/providers/ProviderLogo';
import Sparkline from '@/components/charts/Sparkline';
import WidgetGauge, { arcColor } from '@/components/meters/WidgetGauge';
import ResetCountdown from '@/components/meters/ResetCountdown';
import {
  ModelBreakdownEntry,
  ProviderId,
  ProviderStatus,
  TrendData,
  UsageAlert,
  UsageSnapshot,
} from '@/types';
import { getProviderAccessState, providerAccessDotClass } from '@/utils/providerAccess';

const providerMeta: Record<ProviderId, { label: string; tint: 'claude' | 'codex' | 'gemini'; color: string }> = {
  claude: { label: 'Claude', tint: 'claude', color: '#d97757' },
  codex: { label: 'Codex', tint: 'codex', color: '#10a37f' },
  gemini: { label: 'Gemini', tint: 'gemini', color: '#4285f4' },
};

const widgetWindowLabel = (windowType?: string) => {
  switch (windowType) {
    case 'five_hour': return '5H';
    case 'seven_day': return '7D';
    case 'daily': return 'DAY';
    case 'monthly': return 'MO';
    case 'weekly': return 'WK';
    case 'session': return 'SES';
    default: return 'WIN';
  }
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
  const secondaryPct = secondary ? Math.max(0, Math.min(100, secondary.utilization ?? 0)) : undefined;
  const costToday = trend?.points[trend.points.length - 1]?.cost_usd ?? 0;
  const access = getProviderAccessState(status, snapshot);
  const healthClass = providerAccessDotClass(access.health);

  return (
    <div
      className={`overview-card-v2 glass-${meta.tint}`}
      onClick={onClick}
      title={access.detail}
    >
      {/* Left: Gauge */}
      <div className="overview-card-gauge-wrap">
        <WidgetGauge provider={provider} primaryPct={primaryPct} secondaryPct={secondaryPct} />
      </div>

      {/* Center: Identity + metrics */}
      <div className="overview-card-body">
        <div className="overview-card-title-row">
          <ProviderLogo provider={provider} size={14} />
          <span className="overview-card-title">{meta.label}</span>
          <span className={`nav-tab-dot ${healthClass}`} />
          <span
            className="overview-card-status-badge"
            style={{ color: access.color, borderColor: access.color }}
          >
            {access.label}
          </span>
        </div>

        {/* Usage windows */}
        <div className="overview-card-windows">
          {snapshot ? (
            <>
              <div className="overview-card-window">
                <div className="overview-card-window-head">
                  <span className="widget-provider-window-label">
                    {widgetWindowLabel(primary?.window_type)}
                  </span>
                  <span className="overview-card-pct" style={{ color: arcColor(primaryPct) }}>
                    {primaryPct.toFixed(0)}%
                  </span>
                </div>
                <ResetCountdown resetsAt={primary?.resets_at} className="overview-card-reset" />
              </div>
              {secondaryPct != null && secondary && (
                <div className="overview-card-window">
                  <div className="overview-card-window-head">
                    <span className="widget-provider-window-label">
                      {widgetWindowLabel(secondary?.window_type)}
                    </span>
                    <span className="overview-card-pct" style={{ color: arcColor(secondaryPct) }}>
                      {secondaryPct.toFixed(0)}%
                    </span>
                  </div>
                  <ResetCountdown resetsAt={secondary?.resets_at} className="overview-card-reset" />
                </div>
              )}
            </>
          ) : (
            <span className="widget-provider-empty" title={access.detail}>
              {access.health === 'error' ? 'Unavailable' : 'Awaiting'}
            </span>
          )}
        </div>

        {/* Model + alerts row */}
        {(breakdown.length > 0 || alerts.length > 0) && (
          <div className="overview-card-meta-row">
            {breakdown.length > 0 && (
              <span className="overview-card-model-chip">
                {breakdown[0].model} · {formatTokens(breakdown[0].total_tokens)}
              </span>
            )}
            {alerts.length > 0 && (
              <div className="overview-card-alerts">
                {alerts.slice(0, 2).map((alert) => (
                  <span
                    key={`${alert.window_type}-${alert.threshold_percent}`}
                    className="overview-card-alert-badge"
                    style={{
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
      </div>

      {/* Right: Sparkline + cost */}
      <div className="overview-card-spark-col">
        <div className="overview-card-spark">
          <Sparkline points={trend?.points ?? []} color={meta.color} height={36} />
        </div>
        <div className="overview-card-cost-row">
          <span className="overview-card-cost">${costToday.toFixed(2)}</span>
          <span className="overview-card-cost-label">
            / ${trend?.total_cost_usd.toFixed(2) ?? '0.00'} 30d
          </span>
        </div>
      </div>
    </div>
  );
};

export default OverviewCard;
