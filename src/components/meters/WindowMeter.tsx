import { UsageWindow } from '@/types';
import UsageMeter from '@/components/meters/UsageMeter';

type WindowMeterProps = {
  session?: UsageWindow;
  weekly?: UsageWindow;
  providerTint: 'claude' | 'codex' | 'gemini';
};

const WindowMeter = ({ session, weekly, providerTint }: WindowMeterProps) => {
  return (
    <div style={{ display: 'flex', gap: 12, flexWrap: 'wrap' }}>
      <UsageMeter
        utilization={session?.utilization ?? 0}
        label="Session"
        resetsAt={session?.resets_at}
        providerTint={providerTint}
      />
      <UsageMeter
        utilization={weekly?.utilization ?? 0}
        label="Weekly"
        resetsAt={weekly?.resets_at}
        providerTint={providerTint}
      />
    </div>
  );
};

export default WindowMeter;
