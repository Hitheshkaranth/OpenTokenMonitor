import { UsageWindow } from '@/types';
import UsageMeter from '@/components/meters/UsageMeter';
import { windowLabel, windowValueLabel } from '@/utils/usageWindows';

type WindowMeterProps = {
  primary?: UsageWindow;
  secondary?: UsageWindow;
  providerTint: 'claude' | 'codex' | 'gemini';
};

const WindowMeter = ({ primary, secondary, providerTint }: WindowMeterProps) => {
  return (
    <div style={{ display: 'flex', gap: 12, flexWrap: 'wrap' }}>
      <UsageMeter
        utilization={primary?.utilization ?? 0}
        label={windowLabel(primary)}
        detail={windowValueLabel(primary)}
        note={primary?.note}
        resetsAt={primary?.resets_at}
        providerTint={providerTint}
      />
      <UsageMeter
        utilization={secondary?.utilization ?? 0}
        label={windowLabel(secondary)}
        detail={windowValueLabel(secondary)}
        note={secondary?.note}
        resetsAt={secondary?.resets_at}
        providerTint={providerTint}
      />
    </div>
  );
};

export default WindowMeter;
