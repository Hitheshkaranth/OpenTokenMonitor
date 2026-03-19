import GlassPanel from '@/components/glass/GlassPanel';
import GlassPill from '@/components/glass/GlassPill';
import ProviderLogo from '@/components/providers/ProviderLogo';
import { ModelBreakdownEntry, ProviderId, ProviderStatus, RecentActivityEntry } from '@/types';

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

type WidgetActivityViewProps = {
  provider: ProviderId;
  statuses: Partial<Record<ProviderId, ProviderStatus>>;
  modelBreakdowns: Record<ProviderId, ModelBreakdownEntry[]>;
  recentActivity: Record<ProviderId, RecentActivityEntry[]>;
  onSelectProvider: (provider: ProviderId) => void;
};

const WidgetActivityView = ({
  provider,
  statuses,
  modelBreakdowns,
  recentActivity,
  onSelectProvider,
}: WidgetActivityViewProps) => {
  const meta = providerMeta[provider];
  const topModel = modelBreakdowns[provider][0];
  const entries = recentActivity[provider];

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 6, flex: 1, minHeight: 0, padding: '6px 8px 8px' }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
        {providers.map((id) => (
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
          >
            <span className={`nav-tab-dot ${statuses[id]?.health ? `health-${statuses[id]!.health}` : 'health-unknown'}`} />
            {providerMeta[id].label}
          </GlassPill>
        ))}
      </div>

      <GlassPanel
        tint={meta.tint}
        style={{ display: 'flex', flexDirection: 'column', gap: 6, flex: 1, minHeight: 0, padding: '8px 9px' }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: 8, minWidth: 0 }}>
          <ProviderLogo provider={provider} size={20} />
          <div style={{ display: 'flex', flexDirection: 'column', minWidth: 0 }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: 5 }}>
              <span style={{ fontSize: 12, fontWeight: 700, whiteSpace: 'nowrap' }}>{meta.label}</span>
              <span className={`nav-tab-dot ${statuses[provider]?.health ? `health-${statuses[provider]!.health}` : 'health-unknown'}`} />
            </div>
            <span className="metric-label" style={{ fontSize: 8 }}>
              Recent inputs from local CLI history
            </span>
          </div>
          {topModel && (
            <span
              className="glass-pill"
              style={{
                marginLeft: 'auto',
                fontSize: 8,
                padding: '1px 5px',
                maxWidth: 126,
                overflow: 'hidden',
                textOverflow: 'ellipsis',
                whiteSpace: 'nowrap',
              }}
              title={`${topModel.model} · ${formatTokens(topModel.total_tokens)} tokens`}
            >
              {topModel.model} · {formatTokens(topModel.total_tokens)}
            </span>
          )}
        </div>

        {entries.length === 0 ? (
          <div
            style={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              flex: 1,
              minHeight: 0,
              fontSize: 10,
              color: 'var(--text-secondary)',
              textAlign: 'center',
              lineHeight: 1.4,
              padding: '0 8px',
            }}
          >
            No recent prompts were detected for this provider yet.
          </div>
        ) : (
          <div style={{ display: 'grid', gap: 5, overflowY: 'auto', minHeight: 0, paddingRight: 2 }}>
            {entries.map((entry, index) => (
              <div
                key={`${entry.timestamp}-${entry.session_id ?? index}`}
                style={{
                  display: 'grid',
                  gap: 3,
                  padding: '6px 7px',
                  borderRadius: 10,
                  border: '1px solid rgba(255,255,255,0.08)',
                  background: 'rgba(10, 14, 22, 0.28)',
                }}
                title={entry.cwd ?? entry.prompt}
              >
                <div style={{ display: 'flex', alignItems: 'center', gap: 6, fontSize: 8 }}>
                  <span style={{ fontWeight: 700, color: 'var(--text-primary)' }}>{fallbackTerminalLabel(entry)}</span>
                  <span className="metric-label" style={{ marginLeft: 'auto' }}>
                    {formatAge(entry.timestamp)}
                  </span>
                </div>
                <div
                  style={{
                    fontSize: 10,
                    lineHeight: 1.3,
                    color: 'var(--text-primary)',
                    display: '-webkit-box',
                    WebkitBoxOrient: 'vertical',
                    WebkitLineClamp: 2,
                    overflow: 'hidden',
                  }}
                >
                  {entry.prompt}
                </div>
              </div>
            ))}
          </div>
        )}
      </GlassPanel>
    </div>
  );
};

export default WidgetActivityView;
