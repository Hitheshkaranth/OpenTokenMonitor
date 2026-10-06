import { useEffect } from 'react';
import ProviderLogo from '@/components/providers/ProviderLogo';
import WidgetGauge, { arcColor } from '@/components/meters/WidgetGauge';
import LimitEta, { forecastFor } from '@/components/meters/LimitEta';
import { ModelBreakdownEntry, ProviderId, ProviderStatus, UsageAlert, UsageSnapshot } from '@/types';
import { useUsageStore } from '@/stores/usageStore';
import { useBurnRate, BurnReadout } from '@/hooks/useBurnRate';
import { getProviderAccessState, providerAccessColor } from '@/utils/providerAccess';
import { displayWindows } from '@/utils/usageWindows';
import '@/components/comparison/comparison.css';

const providers: ProviderId[] = ['claude', 'codex', 'antigravity'];

const providerMeta: Record<ProviderId, { label: string; tint: 'claude' | 'codex' | 'antigravity'; color: string }> = {
  claude: { label: 'Claude', tint: 'claude', color: '#d97757' },
  codex: { label: 'Codex', tint: 'codex', color: '#10a37f' },
  antigravity: { label: 'Antigravity', tint: 'antigravity', color: '#4f6bed' },
};

const severityColor: Record<UsageAlert['severity'], string> = {
  warning: '#f59e0b',
  high: '#f97316',
  critical: '#ef4444',
};

const formatTokens = (value: number) => {
  if (value >= 1_000_000_000) return `${(value / 1_000_000_000).toFixed(1)}B`;
  if (value >= 1_000_000) return `${(value / 1_000_000).toFixed(1)}M`;
  if (value >= 1_000) return `${(value / 1_000).toFixed(1)}K`;
  return String(Math.round(value));
};

// Keep large totals narrow enough for the compact window cells.
const formatUsd = (value: number) =>
  value >= 1000 ? `$${Math.round(value).toLocaleString()}` : `$${value.toFixed(2)}`;

// Sum the cost buckets that actually fall inside the selected window so the
// headline number tracks the trend selector rather than everything on record.
const sumCost = (entries: { date: string; estimated_cost_usd: number }[], days: number) => {
  if (!entries || entries.length === 0) return 0;
  const cutoff = Date.now() - days * 24 * 60 * 60 * 1000;
  let total = 0;
  for (const entry of entries) {
    const ts = new Date(entry.date).getTime();
    if (Number.isFinite(ts) && ts >= cutoff) {
      total += entry.estimated_cost_usd;
    }
  }
  return total;
};

const tokenTotal = (breakdown: ModelBreakdownEntry[]) =>
  breakdown.reduce((acc, entry) => acc + (entry.total_tokens ?? 0), 0);

const burnLabel = (burn: BurnReadout) => {
  if (!burn.hasSamples || burn.tokPerSec == null || burn.tokensPerMin == null) return null;
  if (burn.tokensPerMin < 1) return null;
  const rate = burn.tokPerSec >= 1 ? `${burn.tokPerSec.toFixed(0)} tok/s` : `${formatTokens(burn.tokensPerMin)} tok/min`;
  return `${burn.rising ? '↑' : '→'} ${rate}`;
};

type ComparisonRowProps = {
  provider: ProviderId;
  snapshot?: UsageSnapshot;
  cost: number;
  tokens: number;
  alerts: UsageAlert[];
  status?: ProviderStatus;
  trendDays: number;
  onClick?: () => void;
};

// One provider per row, built on the Overview card (overview-card-v2) so the
// comparison reads as the same system: gauge, identity, window cells, chips.
const ComparisonRow = ({ provider, snapshot, cost, tokens, alerts, status, trendDays, onClick }: ComparisonRowProps) => {
  const meta = providerMeta[provider];
  const authState = useUsageStore((s) => s.authStates[provider]);
  const forecasts = useUsageStore((s) => s.forecasts[provider]);
  const access = getProviderAccessState(status, snapshot, authState);
  const burn = burnLabel(useBurnRate(snapshot));

  const fiveHour = snapshot?.windows.find((w) => w.window_type === 'five_hour');
  const [primary] = displayWindows(snapshot);
  const headline = fiveHour ?? primary;
  const pct = Math.max(0, Math.min(100, headline?.utilization ?? 0));

  return (
    <div className={`overview-card-v2 glass-${meta.tint} cmp-row`} onClick={onClick} title={access.detail}>
      <div className="overview-card-gauge-wrap">
        <WidgetGauge provider={provider} primaryPct={pct} />
      </div>

      <div className="overview-card-body">
        <div className="overview-card-title-row">
          <ProviderLogo provider={provider} size={14} />
          <span className="overview-card-title">{meta.label}</span>
          <span
            className="overview-card-status-badge"
            style={{ color: providerAccessColor(access.health), borderColor: providerAccessColor(access.health) }}
          >
            {access.label}
          </span>
          <div className="overview-card-alerts">
            {burn && (
              <span className="overview-card-model-chip" title="Token burn between the last two refreshes">
                {burn}
              </span>
            )}
            {alerts.slice(0, 1).map((alert) => (
              <span
                key={`${alert.window_type}-${alert.threshold_percent}`}
                className="overview-card-alert-badge"
                style={{ color: severityColor[alert.severity], borderColor: severityColor[alert.severity] }}
                title={alert.message}
              >
                {alert.severity}
              </span>
            ))}
          </div>
        </div>

        <div className="overview-card-windows">
          <div className="overview-card-window">
            <div className="overview-card-window-head">
              <span className="widget-provider-window-label">{fiveHour ? '5H' : 'NOW'}</span>
              <span className="overview-card-pct" style={{ color: arcColor(pct) }}>
                {snapshot ? `${pct.toFixed(0)}%` : '—'}
              </span>
            </div>
            <LimitEta
              replacesReset
              resetsAt={headline?.resets_at}
              forecast={forecastFor(forecasts, headline)}
              className="overview-card-reset"
            />
          </div>
          <div className="overview-card-window">
            <span className="overview-card-pct">{formatUsd(cost)}</span>
            <span className="overview-card-cost-label">{trendDays}d spend</span>
          </div>
          <div className="overview-card-window">
            <span className="overview-card-pct">{formatTokens(tokens)}</span>
            <span className="overview-card-cost-label">{trendDays}d tokens</span>
          </div>
        </div>

      </div>
    </div>
  );
};

type ComparisonViewProps = {
  snapshots: Record<ProviderId, UsageSnapshot | undefined>;
  costHistory: Record<ProviderId, { date: string; provider: ProviderId; model: string; estimated_cost_usd: number }[]>;
  modelBreakdowns: Record<ProviderId, ModelBreakdownEntry[]>;
  alerts: Record<ProviderId, UsageAlert[]>;
  statuses: Record<ProviderId, ProviderStatus | undefined>;
  trendDays: number;
  onNavigate?: (provider: ProviderId) => void;
};

const ComparisonView = ({
  snapshots,
  costHistory,
  modelBreakdowns,
  alerts,
  statuses,
  trendDays,
  onNavigate,
}: ComparisonViewProps) => {
  const fetchSnapshot = useUsageStore((s) => s.fetchSnapshot);
  const fetchCostHistory = useUsageStore((s) => s.fetchCostHistory);
  const fetchStatus = useUsageStore((s) => s.fetchStatus);
  const fetchUsageReport = useUsageStore((s) => s.fetchUsageReport);

  useEffect(() => {
    const run = async () => {
      await Promise.all([
        ...providers.map((provider) => Promise.all([fetchSnapshot(provider), fetchStatus(provider), fetchCostHistory(provider, trendDays)])),
        fetchUsageReport(trendDays),
      ]);
    };
    run().catch(() => undefined);
  }, [fetchSnapshot, fetchCostHistory, fetchStatus, fetchUsageReport, trendDays]);

  const rows = providers.map((provider) => ({
    provider,
    cost: sumCost(costHistory[provider] ?? [], trendDays),
    tokens: tokenTotal(modelBreakdowns[provider] ?? []),
  }));
  const totalCost = rows.reduce((sum, r) => sum + r.cost, 0);
  const totalTokens = rows.reduce((sum, r) => sum + r.tokens, 0);
  const totalAlerts = providers.reduce((sum, p) => sum + (alerts[p] ?? []).length, 0);
  const share = (cost: number) => (totalCost > 0 ? cost / totalCost : 0);

  return (
    <div className="cmp-root">
      <div className="proj-header">
        <div className="proj-header-left">
          <span className="proj-header-title">Compare</span>
          <span className="proj-header-subtitle">Usage, spend and burn side by side · last {trendDays}d</span>
        </div>
        <span className="proj-header-badge">{totalAlerts > 0 ? `${totalAlerts} ${totalAlerts === 1 ? 'alert' : 'alerts'}` : `${providers.length} sources`}</span>
      </div>

      {/* Spend share across providers (same chrome as provider-page panels). */}
      <div className="pcard-section-panel">
        <div className="pcard-section-head">
          <span className="pcard-section-title">Spend share</span>
          <div className="pcard-cost-pills">
            <span className="pcard-cost-chip">{formatUsd(totalCost)}</span>
            <span className="pcard-cost-chip">{formatTokens(totalTokens)} tok</span>
          </div>
        </div>
        <div className="pcard-mix-bar" role="img" aria-label="Share of spend by provider">
          {rows
            .filter((r) => r.cost > 0)
            .map((r) => (
              <span
                key={r.provider}
                className="pcard-mix-seg"
                style={{ width: `${share(r.cost) * 100}%`, background: providerMeta[r.provider].color }}
                title={`${providerMeta[r.provider].label} · ${Math.round(share(r.cost) * 100)}%`}
              />
            ))}
        </div>
        <div className="cmp-legend">
          {rows.map((r) => (
            <span key={r.provider} className="cmp-legend-item">
              <span className="pcard-mix-dot" style={{ background: providerMeta[r.provider].color }} />
              <span className="pcard-model-name">{providerMeta[r.provider].label}</span>
              <span className="pcard-model-cost">{Math.round(share(r.cost) * 100)}%</span>
            </span>
          ))}
        </div>
      </div>

      <div className="cmp-rows">
        {rows.map((r) => (
          <ComparisonRow
            key={r.provider}
            provider={r.provider}
            snapshot={snapshots[r.provider]}
            cost={r.cost}
            tokens={r.tokens}
            alerts={alerts[r.provider] ?? []}
            status={statuses[r.provider]}
            trendDays={trendDays}
            onClick={onNavigate ? () => onNavigate(r.provider) : undefined}
          />
        ))}
      </div>
    </div>
  );
};

export default ComparisonView;
