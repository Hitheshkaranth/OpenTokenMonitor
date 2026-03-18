import ProviderLogo from '@/components/providers/ProviderLogo';
import { ProviderId } from '@/types';

type WidgetGaugeProps = {
  provider: ProviderId;
  primaryPct: number;
  secondaryPct?: number;
};

/* ── Apple Activity Ring — sized for widget cards ── */
const SIZE = 72;
const CENTER = SIZE / 2;
const OUTER_R = 30;
const INNER_R = 22;
const OUTER_STROKE = 5.5;
const INNER_STROKE = 4.5;

export const arcColor = (pct: number) => {
  if (pct > 80) return '#ef4444';
  if (pct >= 50) return '#f59e0b';
  return '#22c55e';
};

const trackColor = (pct: number) => {
  if (pct > 80) return 'rgba(239, 68, 68, 0.18)';
  if (pct >= 50) return 'rgba(245, 158, 11, 0.18)';
  return 'rgba(34, 197, 94, 0.18)';
};

const WidgetGauge = ({ provider, primaryPct, secondaryPct }: WidgetGaugeProps) => {
  const outerC = 2 * Math.PI * OUTER_R;
  const innerC = 2 * Math.PI * INNER_R;
  const outerOffset = outerC - (primaryPct / 100) * outerC;
  const innerOffset = secondaryPct != null
    ? innerC - (secondaryPct / 100) * innerC
    : innerC;

  return (
    <div style={{ position: 'relative', width: SIZE, height: SIZE, flexShrink: 0 }}>
      <svg width={SIZE} height={SIZE} viewBox={`0 0 ${SIZE} ${SIZE}`}>
        {/* Outer track */}
        <circle cx={CENTER} cy={CENTER} r={OUTER_R}
          fill="none" stroke={trackColor(primaryPct)} strokeWidth={OUTER_STROKE} />
        {/* Outer ring */}
        <circle cx={CENTER} cy={CENTER} r={OUTER_R}
          fill="none" stroke={arcColor(primaryPct)}
          strokeWidth={OUTER_STROKE} strokeLinecap="round"
          strokeDasharray={outerC} strokeDashoffset={outerOffset}
          transform={`rotate(-90 ${CENTER} ${CENTER})`}
          style={{
            transition: 'stroke-dashoffset .45s cubic-bezier(.22,.9,.24,1)',
            filter: `drop-shadow(0 0 3px ${arcColor(primaryPct)}66)`,
          }}
        />
        {/* Inner track */}
        <circle cx={CENTER} cy={CENTER} r={INNER_R}
          fill="none"
          stroke={secondaryPct != null ? trackColor(secondaryPct) : 'rgba(255,255,255,0.06)'}
          strokeWidth={INNER_STROKE} />
        {/* Inner ring */}
        {secondaryPct != null && (
          <circle cx={CENTER} cy={CENTER} r={INNER_R}
            fill="none" stroke={arcColor(secondaryPct)}
            strokeWidth={INNER_STROKE} strokeLinecap="round"
            strokeDasharray={innerC} strokeDashoffset={innerOffset}
            transform={`rotate(-90 ${CENTER} ${CENTER})`}
            style={{
              transition: 'stroke-dashoffset .45s cubic-bezier(.22,.9,.24,1)',
              filter: `drop-shadow(0 0 2px ${arcColor(secondaryPct)}66)`,
            }}
          />
        )}
      </svg>
      {/* Provider logo centered */}
      <div style={{ position: 'absolute', top: '50%', left: '50%', transform: 'translate(-50%, -50%)' }}>
        <ProviderLogo provider={provider} size={28} />
      </div>
    </div>
  );
};

export default WidgetGauge;
