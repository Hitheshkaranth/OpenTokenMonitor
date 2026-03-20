import RecentActivitySlides from '@/components/activity/RecentActivitySlides';
import GlassPanel from '@/components/glass/GlassPanel';
import GlassPill from '@/components/glass/GlassPill';
import ProviderLogo from '@/components/providers/ProviderLogo';
import { ModelBreakdownEntry, ProviderId, ProviderStatus, RecentActivityEntry, UsageSnapshot } from '@/types';
import { getProviderAccessState, providerAccessDotClass } from '@/utils/providerAccess';

const providerMeta: Record<ProviderId, { label: string; tint: 'claude' | 'codex' | 'gemini' }> = {
  claude: { label: 'Claude', tint: 'claude' },
  codex: { label: 'Codex', tint: 'codex' },
  gemini: { label: 'Gemini', tint: 'gemini' },
};

const providers: ProviderId[] = ['claude', 'codex', 'gemini'];

const formatTokens = (value: number) => {
  if (value >= 1_000_000) return `${(value / 1_000_000).toFixed(1)}M`;
  if (value >= 1_000) return `${(value / 1_000).toFixed(1)}K`;
  return String(value);
};

type WidgetActivityViewProps = {
  provider: ProviderId;
  statuses: Partial<Record<ProviderId, ProviderStatus>>;
  snapshots: Record<ProviderId, UsageSnapshot | undefined>;
  modelBreakdowns: Record<ProviderId, ModelBreakdownEntry[]>;
  recentActivity: Record<ProviderId, RecentActivityEntry[]>;
  onSelectProvider: (provider: ProviderId) => void;
};

const WidgetActivityView = ({
  provider,
  statuses,
  snapshots,
  modelBreakdowns,
  recentActivity,
  onSelectProvider,
}: WidgetActivityViewProps) => {
  const meta = providerMeta[provider];
  const topModel = modelBreakdowns[provider][0];
  const entries = recentActivity[provider];
  const activeAccess = getProviderAccessState(statuses[provider], snapshots[provider]);

  return (
    <div className="widget-activity-root">
      <div className="widget-activity-pill-row">
        {providers.map((id) => {
          const access = getProviderAccessState(statuses[id], snapshots[id]);
          return (
            <GlassPill
              key={id}
              active={id === provider}
              onClick={() => onSelectProvider(id)}
              style={{
                display: 'inline-flex',
                alignItems: 'center',
                gap: 4,
                height: 22,
                padding: '0 8px',
                fontSize: 9,
                fontWeight: id === provider ? 700 : 600,
                whiteSpace: 'nowrap',
              }}
              title={access.detail}
            >
              <span className={`nav-tab-dot ${providerAccessDotClass(access.health)}`} />
              {providerMeta[id].label}
            </GlassPill>
          );
        })}
      </div>

      <GlassPanel
        className="widget-activity-panel"
        tint={meta.tint}
        title={activeAccess.detail}
      >
        <div className="widget-activity-header">
          <ProviderLogo provider={provider} size={20} />
          <div className="widget-activity-provider-copy">
            <div className="widget-activity-title-row">
              <span className="widget-activity-title">{meta.label}</span>
              <span className={`nav-tab-dot ${providerAccessDotClass(activeAccess.health)}`} />
            </div>
            <span className="metric-label widget-activity-caption">
              {activeAccess.health === 'active' ? 'Recent conversations' : activeAccess.detail}
            </span>
          </div>
          {topModel && (
            <span
              className="glass-pill widget-activity-top-model"
              title={`${topModel.model} / ${formatTokens(topModel.total_tokens)} tokens`}
            >
              {topModel.model} / {formatTokens(topModel.total_tokens)}
            </span>
          )}
        </div>

        <div
          className="glass-pill widget-activity-status"
          style={{ color: activeAccess.color, borderColor: activeAccess.color }}
        >
          {activeAccess.label}
        </div>

        <RecentActivitySlides
          entries={entries}
          emptyMessage="No recent conversations detected yet."
          variant="widget"
          resetKey={provider}
        />
      </GlassPanel>
    </div>
  );
};

export default WidgetActivityView;
