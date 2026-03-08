import { RefreshCw } from 'lucide-react';
import WindowMeter from '@/components/meters/WindowMeter';
import CostTrendChart from '@/components/charts/CostTrendChart';
import GlassPill from '@/components/glass/GlassPill';
import ProviderLogo from '@/components/providers/ProviderLogo';
import { ProviderId, TrendData, UsageSnapshot } from '@/types';

const providerMeta: Record<ProviderId, { name: string; tint: 'claude' | 'codex' | 'gemini'; color: string }> = {
  claude: { name: 'Claude', tint: 'claude', color: '#d97757' },
  codex: { name: 'Codex', tint: 'codex', color: '#10a37f' },
  gemini: { name: 'Gemini', tint: 'gemini', color: '#4285f4' },
};

type ProviderCardProps = {
  snapshot?: UsageSnapshot;
  trend?: TrendData;
  onRefresh: () => void;
};

const ProviderCard = ({ snapshot, trend, onRefresh }: ProviderCardProps) => {
  if (!snapshot) {
    return <div className="glass-panel" style={{ padding: 14 }}>No provider snapshot yet.</div>;
  }

  const meta = providerMeta[snapshot.provider];
  const session = snapshot.windows.find((w) => w.window_type === 'session' || w.window_type === 'five_hour' || w.window_type === 'daily');
  const weekly = snapshot.windows.find((w) => w.window_type === 'weekly' || w.window_type === 'seven_day');
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

      <WindowMeter session={session} weekly={weekly} providerTint={meta.tint} />

      <div style={{ display: 'flex', gap: 8, flexWrap: 'wrap' }}>
        <span className="glass-pill">Today ${costToday.toFixed(2)}</span>
        <span className="glass-pill">30d ${trend?.total_cost_usd.toFixed(2) ?? '0.00'}</span>
        <span className="glass-pill">Updated {new Date(snapshot.fetched_at).toLocaleTimeString()}</span>
      </div>

      <CostTrendChart points={trend?.points ?? []} color={meta.color} />
    </div>
  );
};

export default ProviderCard;
