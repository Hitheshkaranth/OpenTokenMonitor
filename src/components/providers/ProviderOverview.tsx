import GlassPanel from '@/components/glass/GlassPanel';
import ProviderLogo from '@/components/providers/ProviderLogo';
import { ProviderId, TrendData, UsageSnapshot } from '@/types';

const providerMeta: Record<ProviderId, { label: string; tint: 'claude' | 'codex' | 'gemini' }> = {
  claude: { label: 'Claude', tint: 'claude' },
  codex: { label: 'Codex', tint: 'codex' },
  gemini: { label: 'Gemini', tint: 'gemini' },
};

const pctUsed = (utilization?: number) => Math.max(0, Math.min(100, utilization ?? 0));

const formatTokens = (n?: number | null) => {
  if (n == null) return '—';
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(0)}K`;
  return String(n);
};

const secsToHours = (secs?: number) => {
  if (!secs || secs <= 0) return 0;
  return Math.max(0, Math.ceil(secs / 3600));
};

const secsToDays = (secs?: number) => {
  if (!secs || secs <= 0) return 0;
  return Math.max(0, Math.ceil(secs / 86400));
};

type ProviderOverviewProps = {
  snapshots: Record<ProviderId, UsageSnapshot | undefined>;
  trends: Record<ProviderId, TrendData | undefined>;
};

const ProviderOverview = ({ snapshots, trends }: ProviderOverviewProps) => {
  const rows: ProviderId[] = ['claude', 'codex', 'gemini'];

  return (
    <div style={{ display: 'grid', gap: 10 }}>
      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, minmax(0, 1fr))', gap: 10 }}>
        {rows.map((provider) => {
          const snapshot = snapshots[provider];
          const trend = trends[provider];
          const session = snapshot?.windows.find((w) => w.window_type === 'session' || w.window_type === 'five_hour' || w.window_type === 'daily');
          const weekly = snapshot?.windows.find((w) => w.window_type === 'weekly' || w.window_type === 'seven_day');
          const sessionUsedPct = pctUsed(session?.utilization);
          const weeklyUsedPct = pctUsed(weekly?.utilization);
          const hoursLeft = secsToHours(session?.reset_countdown_secs);
          const daysLeft = secsToDays(weekly?.reset_countdown_secs);

          return (
            <GlassPanel key={provider} tint={providerMeta[provider].tint} className="hover-lift" style={{ padding: 10, minWidth: 0, height: 182 }}>
              <div className="provider-name" style={{ display: 'flex', alignItems: 'center', gap: 6, minWidth: 0 }}>
                <ProviderLogo provider={provider} size={15} />
                <span style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap', fontSize: 14, lineHeight: 1.1 }}>
                  {providerMeta[provider].label}
                </span>
              </div>

              <div style={{ display: 'grid', gap: 8, marginTop: 8, minWidth: 0 }}>
                <div style={{ display: 'grid', gap: 4 }}>
                  <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'end', gap: 6, minWidth: 0 }}>
                    <span style={{ fontFamily: 'var(--font-mono)', fontSize: 20, fontWeight: 700, lineHeight: 1, color: 'var(--text-primary)' }}>
                      {sessionUsedPct.toFixed(0)}%
                    </span>
                    <span className="metric-label" style={{ fontSize: 10, lineHeight: 1.1, letterSpacing: '.03em', whiteSpace: 'nowrap' }}>
                      {hoursLeft}h left
                    </span>
                  </div>
                  <div className="metric-label" style={{ fontSize: 10, lineHeight: 1.1, letterSpacing: '.04em', color: 'var(--text-secondary)' }}>
                    Session · {formatTokens(session?.used)} / {formatTokens(session?.limit)}
                  </div>
                  <div className="glass-pill" style={{ width: '100%', padding: 0, height: 8, overflow: 'hidden', display: 'block' }}>
                    <div style={{ width: `${sessionUsedPct}%`, height: '100%', borderRadius: 999, background: 'linear-gradient(90deg, #f59e0b, #fb923c)', transition: 'width .45s cubic-bezier(.22,.9,.24,1)' }} />
                  </div>
                </div>

                <div style={{ display: 'grid', gap: 4 }}>
                  <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'end', gap: 6, minWidth: 0 }}>
                    <span style={{ fontFamily: 'var(--font-mono)', fontSize: 20, fontWeight: 700, lineHeight: 1, color: 'var(--text-primary)' }}>
                      {weeklyUsedPct.toFixed(0)}%
                    </span>
                    <span className="metric-label" style={{ fontSize: 10, lineHeight: 1.1, letterSpacing: '.03em', whiteSpace: 'nowrap' }}>
                      {daysLeft}d left
                    </span>
                  </div>
                  <div className="metric-label" style={{ fontSize: 10, lineHeight: 1.1, letterSpacing: '.04em', color: 'var(--text-secondary)' }}>
                    Weekly · {formatTokens(weekly?.used)} / {formatTokens(weekly?.limit)}
                  </div>
                  <div className="glass-pill" style={{ width: '100%', padding: 0, height: 8, overflow: 'hidden', display: 'block' }}>
                    <div style={{ width: `${weeklyUsedPct}%`, height: '100%', borderRadius: 999, background: 'linear-gradient(90deg, #ef4444, #f87171)', transition: 'width .45s cubic-bezier(.22,.9,.24,1)' }} />
                  </div>
                </div>
              </div>

              <div style={{ display: 'flex', justifyContent: 'space-between', gap: 8, marginTop: 8, minWidth: 0 }}>
                <div className="metric-label" style={{ fontSize: 10, lineHeight: 1.1, whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>
                  {snapshot?.source ?? 'none'}
                </div>
                <div className="metric-label" style={{ fontSize: 10, lineHeight: 1.1, whiteSpace: 'nowrap' }}>
                  ${trend?.total_cost_usd.toFixed(2) ?? '0.00'}
                </div>
              </div>
            </GlassPanel>
          );
        })}
      </div>
    </div>
  );
};

export default ProviderOverview;
