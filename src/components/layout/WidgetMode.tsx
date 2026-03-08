import GlassPanel from '@/components/glass/GlassPanel';
import ProviderLogo from '@/components/providers/ProviderLogo';
import { ProviderId, UsageSnapshot } from '@/types';

const names: Record<ProviderId, string> = {
  claude: 'Claude',
  codex: 'Codex',
  gemini: 'Gemini',
};

type WidgetModeProps = {
  snapshots: Record<ProviderId, UsageSnapshot | undefined>;
  onExpand: () => void;
};

const WidgetMode = ({ snapshots, onExpand }: WidgetModeProps) => {
  const providers: ProviderId[] = ['claude', 'codex', 'gemini'];

  return (
    <GlassPanel className="hover-lift" style={{ height: '100%', padding: 12, display: 'grid', gap: 8 }}>
      <button className="glass-pill" style={{ justifySelf: 'start', cursor: 'pointer' }} onClick={onExpand}>
        Expand Dashboard
      </button>
      {providers.map((provider) => {
        const top = snapshots[provider]?.windows[0];
        return (
          <div key={provider} className="glass-pill" style={{ justifyContent: 'space-between' }}>
            <span style={{ display: 'inline-flex', alignItems: 'center', gap: 6 }}>
              <ProviderLogo provider={provider} size={13} />
              {names[provider]}
            </span>
            <strong>{top?.utilization.toFixed(0) ?? '0'}%</strong>
          </div>
        );
      })}
    </GlassPanel>
  );
};

export default WidgetMode;
