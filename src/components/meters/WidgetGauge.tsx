import { useId } from 'react';
import ProviderLogo from '@/components/providers/ProviderLogo';
import { ProviderId } from '@/types';

type WidgetGaugeProps = {
  provider: ProviderId;
  primaryPct: number;
  secondaryPct?: number;
};

/* Apple Activity Ring sized for widget cards */
const SIZE = 66;
const CENTER = SIZE / 2;
const OUTER_R = 27;
const INNER_R = 19.5;
const OUTER_STROKE = 5.2;
const INNER_STROKE = 4.2;
const START_CAP_R = 2.2;

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

const ringPalette = (pct: number) => {
  if (pct > 80) {
    return {
      start: '#ff8aa0',
      end: '#ff453a',
      glow: '#ff667f',
      cap: '#ffd8df',
    };
  }
  if (pct >= 50) {
    return {
      start: '#ffe07a',
      end: '#ff9f0a',
      glow: '#ffbd2f',
      cap: '#fff0b3',
    };
  }
  return {
    start: '#76f7a5',
    end: '#30d158',
    glow: '#43e07a',
    cap: '#dcffe7',
  };
};

const arcEndPoint = (radius: number, pct: number) => {
  const angle = ((pct / 100) * 360 - 90) * (Math.PI / 180);
  return {
    x: CENTER + radius * Math.cos(angle),
    y: CENTER + radius * Math.sin(angle),
  };
};

const startCapPoint = (radius: number) => ({
  x: CENTER,
  y: CENTER - radius,
});

const WidgetGauge = ({ provider, primaryPct, secondaryPct }: WidgetGaugeProps) => {
  const idBase = useId().replace(/:/g, '');
  const outerC = 2 * Math.PI * OUTER_R;
  const innerC = 2 * Math.PI * INNER_R;
  const outerOffset = outerC - (primaryPct / 100) * outerC;
  const innerOffset = secondaryPct != null
    ? innerC - (secondaryPct / 100) * innerC
    : innerC;
  const outerPalette = ringPalette(primaryPct);
  const innerPalette = secondaryPct != null ? ringPalette(secondaryPct) : undefined;
  const outerCap = arcEndPoint(OUTER_R, primaryPct);
  const innerCap = secondaryPct != null ? arcEndPoint(INNER_R, secondaryPct) : undefined;
  const outerStartCap = startCapPoint(OUTER_R);
  const innerStartCap = startCapPoint(INNER_R);
  const outerGradientId = `${idBase}-outer-gradient`;
  const innerGradientId = `${idBase}-inner-gradient`;

  return (
    <div style={{ position: 'relative', width: SIZE, height: SIZE, flexShrink: 0 }}>
      <svg width={SIZE} height={SIZE} viewBox={`0 0 ${SIZE} ${SIZE}`}>
        <defs>
          <linearGradient id={outerGradientId} x1="0%" y1="0%" x2="100%" y2="100%">
            <stop offset="0%" stopColor={outerPalette.start} />
            <stop offset="100%" stopColor={outerPalette.end} />
          </linearGradient>
          {innerPalette && (
            <linearGradient id={innerGradientId} x1="100%" y1="0%" x2="0%" y2="100%">
              <stop offset="0%" stopColor={innerPalette.start} />
              <stop offset="100%" stopColor={innerPalette.end} />
            </linearGradient>
          )}
        </defs>
        <circle
          cx={CENTER}
          cy={CENTER}
          r={29.5}
          fill="var(--widget-gauge-plate-fill)"
          stroke="var(--widget-gauge-plate-stroke)"
          strokeWidth="1"
        />
        <circle cx={CENTER} cy={CENTER} r={OUTER_R}
          fill="none" stroke={trackColor(primaryPct)} strokeWidth={OUTER_STROKE} />
        <circle
          cx={outerStartCap.x}
          cy={outerStartCap.y}
          r={START_CAP_R}
          fill={outerPalette.start}
          opacity={0.92}
        />
        <circle cx={CENTER} cy={CENTER} r={OUTER_R}
          fill="none" stroke={`url(#${outerGradientId})`}
          strokeWidth={OUTER_STROKE} strokeLinecap="round"
          strokeDasharray={outerC} strokeDashoffset={outerOffset}
          transform={`rotate(-90 ${CENTER} ${CENTER})`}
          style={{
            transition: 'stroke-dashoffset .45s cubic-bezier(.22,.9,.24,1)',
            filter: `drop-shadow(0 0 4px ${outerPalette.glow}66)`,
          }}
        />
        {primaryPct > 0.5 && (
          <circle
            cx={outerCap.x}
            cy={outerCap.y}
            r={3.1}
            fill={outerPalette.cap}
            style={{ filter: `drop-shadow(0 0 3px ${outerPalette.glow}88)` }}
          />
        )}
        <circle cx={CENTER} cy={CENTER} r={INNER_R}
          fill="none"
          stroke={secondaryPct != null ? trackColor(secondaryPct) : 'var(--widget-gauge-track-empty)'}
          strokeWidth={INNER_STROKE} />
        {secondaryPct != null && (
          <>
            <circle
              cx={innerStartCap.x}
              cy={innerStartCap.y}
              r={1.8}
              fill={innerPalette?.start}
              opacity={0.88}
            />
            <circle cx={CENTER} cy={CENTER} r={INNER_R}
              fill="none" stroke={`url(#${innerGradientId})`}
              strokeWidth={INNER_STROKE} strokeLinecap="round"
              strokeDasharray={innerC} strokeDashoffset={innerOffset}
              transform={`rotate(-90 ${CENTER} ${CENTER})`}
              style={{
                transition: 'stroke-dashoffset .45s cubic-bezier(.22,.9,.24,1)',
                filter: `drop-shadow(0 0 3px ${innerPalette?.glow ?? '#ffffff'}66)`,
              }}
            />
            {secondaryPct > 0.5 && innerCap && (
              <circle
                cx={innerCap.x}
                cy={innerCap.y}
                r={2.5}
                fill={innerPalette?.cap}
                style={{ filter: `drop-shadow(0 0 2px ${innerPalette?.glow ?? '#ffffff'}88)` }}
              />
            )}
          </>
        )}
        <circle
          cx={CENTER}
          cy={CENTER}
          r={13}
          fill="var(--widget-gauge-core-fill)"
          stroke="var(--widget-gauge-core-stroke)"
          strokeWidth="1.1"
        />
        <circle
          cx={CENTER}
          cy={CENTER}
          r={10}
          fill="var(--widget-gauge-core-inner)"
        />
      </svg>
      <div
        style={{
          position: 'absolute',
          inset: 0,
          display: 'grid',
          placeItems: 'center',
          pointerEvents: 'none',
        }}
      >
        <ProviderLogo provider={provider} size={18} variant="widget-core" />
      </div>
    </div>
  );
};

export default WidgetGauge;
