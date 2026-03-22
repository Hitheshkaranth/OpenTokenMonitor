import { useMemo, useState } from 'react';
import { History, RefreshCw } from 'lucide-react';
import CostTrendChart from '@/components/charts/CostTrendChart';
import RecentActivitySlides from '@/components/activity/RecentActivitySlides';
import ProviderLogo from '@/components/providers/ProviderLogo';
import WidgetGauge, { arcColor } from '@/components/meters/WidgetGauge';
import ResetCountdown from '@/components/meters/ResetCountdown';
import UsageBar from '@/components/meters/UsageBar';
import {
  CostEntry,
  ModelBreakdownEntry,
  ProviderId,
  ProviderStatus,
  RecentActivityEntry,
  TrendData,
  UsageAlert,
  UsageSnapshot,
} from '@/types';
import { getProviderAccessState, providerAccessDotClass } from '@/utils/providerAccess';
import { displayWindows, windowLabel, windowValueLabel } from '@/utils/usageWindows';
import { buildProjectSummaries } from '@/utils/projectActivity';

const providerMeta: Record<ProviderId, { name: string; tint: 'claude' | 'codex' | 'gemini'; color: string; accent: string }> = {
  claude: { name: 'Claude', tint: 'claude', color: '#d97757', accent: '217 119 87' },
  codex: { name: 'Codex', tint: 'codex', color: '#10a37f', accent: '16 163 127' },
  gemini: { name: 'Gemini', tint: 'gemini', color: '#4285f4', accent: '66 133 244' },
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
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  return `${days}d ago`;
};

type ProviderCardProps = {
  snapshot?: UsageSnapshot;
  trend?: TrendData;
  breakdown?: ModelBreakdownEntry[];
  recentActivity?: RecentActivityEntry[];
  allRecentActivity: Record<ProviderId, RecentActivityEntry[]>;
  costHistory: Record<ProviderId, CostEntry[]>;
  alerts?: UsageAlert[];
  status?: ProviderStatus;
  onRefresh: () => void;
};

const ProviderCard = ({
  snapshot,
  trend,
  breakdown = [],
  recentActivity = [],
  allRecentActivity,
  costHistory,
  alerts = [],
  status,
  onRefresh,
}: ProviderCardProps) => {
  const [showRecent, setShowRecent] = useState(false);
  const access = getProviderAccessState(status, snapshot);

  const providerId = snapshot?.provider;

  // Build project summaries filtered for this provider
  const providerProjects = useMemo(() => {
    if (!providerId) return [];
    const providerOnlyActivity: Record<ProviderId, RecentActivityEntry[]> = {
      claude: providerId === 'claude' ? allRecentActivity.claude ?? [] : [],
      codex: providerId === 'codex' ? allRecentActivity.codex ?? [] : [],
      gemini: providerId === 'gemini' ? allRecentActivity.gemini ?? [] : [],
    };
    const providerOnlyCostHistory: Record<ProviderId, CostEntry[]> = {
      claude: providerId === 'claude' ? costHistory.claude ?? [] : [],
      codex: providerId === 'codex' ? costHistory.codex ?? [] : [],
      gemini: providerId === 'gemini' ? costHistory.gemini ?? [] : [],
    };
    return buildProjectSummaries(providerOnlyActivity, providerOnlyCostHistory, {
      maxProjects: 12,
      maxCommandsPerProject: 4,
    });
  }, [providerId, allRecentActivity, costHistory]);

  // Count active sessions (entries in last 15 minutes)
  const activeSessionCount = useMemo(() => {
    if (!providerId) return 0;
    const cutoff = Date.now() - 15 * 60 * 1000;
    const entries = allRecentActivity[providerId] ?? [];
    const activeCwds = new Set<string>();
    for (const e of entries) {
      if (new Date(e.timestamp).getTime() > cutoff && e.cwd) {
        activeCwds.add(e.cwd);
      }
    }
    return activeCwds.size;
  }, [providerId, allRecentActivity]);

  if (!snapshot || !providerId) {
    return (
      <div className="pcard-empty">
        <div className="pcard-empty-label" style={{ color: access.color }}>{access.label}</div>
        <div className="pcard-empty-detail">{access.detail}</div>
      </div>
    );
  }

  const meta = providerMeta[providerId];
  const [primary, secondary] = displayWindows(snapshot);
  const primaryPct = Math.max(0, Math.min(100, primary?.utilization ?? 0));
  const secondaryPct = secondary ? Math.max(0, Math.min(100, secondary.utilization ?? 0)) : undefined;
  const costToday = trend?.points[trend.points.length - 1]?.cost_usd ?? 0;

  return (
    <div
      className="pcard-root"
      style={{ '--widget-accent': meta.accent } as React.CSSProperties}
    >
      {/* Top row: identity + actions */}
      <div className="pcard-top">
        <div className="pcard-identity">
          <ProviderLogo provider={providerId} size={18} />
          <span className="pcard-name">{meta.name}</span>
          <span className={`nav-tab-dot ${providerAccessDotClass(access.health)}`} />
          <span className="pcard-source">{snapshot.source}</span>
          <span
            className="pcard-status-badge"
            style={{ color: access.color, borderColor: access.color }}
          >
            {access.label}
          </span>
          {activeSessionCount > 0 && (
            <span className="pcard-active-badge">
              <span className="pcard-active-dot" />
              {activeSessionCount} active
            </span>
          )}
        </div>
        <div className="pcard-actions">
          <button
            className="pcard-action-btn"
            onClick={() => setShowRecent((v) => !v)}
            title="Toggle recent conversations"
            style={showRecent ? { borderColor: 'var(--control-border-strong)' } : undefined}
          >
            <History size={11} />
          </button>
          <button className="pcard-action-btn" onClick={onRefresh} title="Refresh provider">
            <RefreshCw size={11} />
          </button>
        </div>
      </div>

      {/* Main content: two columns */}
      <div className="pcard-body">
        {/* Left column: gauge + usage + projects */}
        <div className="pcard-left">
          <div className="pcard-gauge-section">
            <div className="pcard-gauge-wrap">
              <WidgetGauge provider={providerId} primaryPct={primaryPct} secondaryPct={secondaryPct} />
            </div>
            <div className="pcard-gauge-legends">
              <div className="pcard-window-compact">
                <span className="pcard-window-label">{windowLabel(primary)}</span>
                <span className="pcard-window-pct" style={{ color: arcColor(primaryPct) }}>
                  {primaryPct.toFixed(0)}%
                </span>
                <ResetCountdown resetsAt={primary?.resets_at} className="pcard-reset" />
              </div>
              {secondary && secondaryPct != null && (
                <div className="pcard-window-compact">
                  <span className="pcard-window-label">{windowLabel(secondary)}</span>
                  <span className="pcard-window-pct" style={{ color: arcColor(secondaryPct) }}>
                    {secondaryPct.toFixed(0)}%
                  </span>
                  <ResetCountdown resetsAt={secondary?.resets_at} className="pcard-reset" />
                </div>
              )}
            </div>
          </div>

          {/* Usage bars */}
          <div className="pcard-usage-panel">
            <div className="pcard-usage-row">
              <UsageBar pct={primaryPct} label={windowLabel(primary)} />
              <span className="pcard-usage-detail">{windowValueLabel(primary)}</span>
              {primary?.note && <span className="pcard-usage-note">{primary.note}</span>}
            </div>
            {secondary && (
              <div className="pcard-usage-row">
                <UsageBar pct={secondaryPct ?? 0} label={windowLabel(secondary)} />
                <span className="pcard-usage-detail">{windowValueLabel(secondary)}</span>
              </div>
            )}
          </div>

          {/* Alerts */}
          {alerts.length > 0 && (
            <div className="pcard-alerts">
              {alerts.map((alert) => (
                <span
                  key={`${alert.window_type}-${alert.threshold_percent}`}
                  className="pcard-alert-chip"
                  style={{ color: severityColor[alert.severity], borderColor: severityColor[alert.severity] }}
                >
                  {alert.window_type} {alert.severity} {alert.utilization.toFixed(0)}%
                </span>
              ))}
            </div>
          )}
        </div>

        {/* Right column: cost chart + models */}
        <div className="pcard-right">
          <div className="pcard-section-panel">
            <div className="pcard-section-head">
              <span className="pcard-section-title">Cost</span>
              <div className="pcard-cost-pills">
                <span className="pcard-cost-chip">Today ${costToday.toFixed(2)}</span>
                <span className="pcard-cost-chip">30d ${trend?.total_cost_usd.toFixed(2) ?? '0.00'}</span>
              </div>
            </div>
            <CostTrendChart points={trend?.points ?? []} color={meta.color} compact />
          </div>

          <div className="pcard-section-panel">
            <span className="pcard-section-title">Models</span>
            {breakdown.length === 0 ? (
              <span className="pcard-empty-detail" style={{ fontSize: 8, marginTop: 2 }}>No per-model data yet.</span>
            ) : (
              <div className="pcard-model-list">
                {breakdown.map((entry) => (
                  <div key={entry.model} className="pcard-model-row">
                    <span className="pcard-model-name">{entry.model}</span>
                    <span className="pcard-model-stat">{formatTokens(entry.input_tokens)}in</span>
                    <span className="pcard-model-stat">{formatTokens(entry.output_tokens)}out</span>
                    <span className="pcard-model-cost">${entry.estimated_cost_usd.toFixed(2)}</span>
                  </div>
                ))}
              </div>
            )}
          </div>
        </div>
      </div>

      {/* Projects section */}
      <div className="pcard-section-panel pcard-projects-section">
        <div className="pcard-section-head">
          <span className="pcard-section-title">Projects</span>
          <span className="pcard-cost-chip">{providerProjects.length} projects</span>
        </div>
        {providerProjects.length === 0 ? (
          <div className="pcard-proj-empty">No project activity detected for this provider yet.</div>
        ) : (
          <div className="pcard-proj-grid soft-scroll">
            {providerProjects.map((project) => {
              const providerCommands = project.commands;
              const providerCost = project.estimated_cost_usd;
              // Check if project has recent activity (last 15 min)
              const cutoff = Date.now() - 15 * 60 * 1000;
              const isActive = providerCommands.some((c) => new Date(c.timestamp).getTime() > cutoff);

              return (
                <div key={project.id} className="pcard-proj-card">
                  <div className="pcard-proj-header">
                    <div className="pcard-proj-title-col">
                      <div className="pcard-proj-title-row">
                        <span className="pcard-proj-title">{project.label}</span>
                        {isActive && (
                          <span className="pcard-proj-active-dot" title="Active in last 15 min" />
                        )}
                      </div>
                      <span className="pcard-proj-path" title={project.path ?? ''}>
                        {project.path ?? 'session activity'}
                      </span>
                    </div>
                    <span className="pcard-proj-cost">${providerCost.toFixed(2)}</span>
                  </div>

                  <div className="pcard-proj-stats">
                    <span className="pcard-proj-stat">
                      <span className="pcard-proj-stat-val">{providerCommands.length}</span>
                      <span className="pcard-proj-stat-label">requests</span>
                    </span>
                    <span className="pcard-proj-stat">
                      <span className="pcard-proj-stat-val">{formatTokens(project.estimated_tokens)}</span>
                      <span className="pcard-proj-stat-label">tokens</span>
                    </span>
                    <span className="pcard-proj-stat">
                      <span className="pcard-proj-stat-val">{project.models.length}</span>
                      <span className="pcard-proj-stat-label">models</span>
                    </span>
                    <span className="pcard-proj-stat">
                      <span className="pcard-proj-stat-val">{formatAge(project.latest_timestamp)}</span>
                      <span className="pcard-proj-stat-label">last seen</span>
                    </span>
                  </div>

                  {project.models.length > 0 && (
                    <div className="pcard-proj-models">
                      {project.models.slice(0, 3).map((model) => (
                        <span key={model} className="pcard-proj-model-chip" title={model}>{model}</span>
                      ))}
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        )}
      </div>

      {/* Recent conversations (expandable) */}
      {showRecent && (
        <div className="pcard-section-panel pcard-recent">
          <div className="pcard-section-head">
            <span className="pcard-section-title">Recent Conversations</span>
            {breakdown[0] && (
              <span className="pcard-cost-chip" title={`${breakdown[0].model} / ${formatTokens(breakdown[0].total_tokens)} tokens`}>
                {breakdown[0].model} / {formatTokens(breakdown[0].total_tokens)}
              </span>
            )}
          </div>
          <RecentActivitySlides
            entries={recentActivity}
            emptyMessage="No recent conversations detected for this provider yet."
            resetKey={providerId}
          />
        </div>
      )}
    </div>
  );
};

export default ProviderCard;
