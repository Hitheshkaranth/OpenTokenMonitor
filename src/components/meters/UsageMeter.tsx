import ResetCountdown from '@/components/meters/ResetCountdown';

type UsageMeterProps = {
  utilization: number;
  label: string;
  detail?: string;
  note?: string;
  resetsAt?: string;
  providerTint: 'claude' | 'codex' | 'gemini';
  size?: number;
};

const arcColor = (value: number) => {
  if (value > 80) return '#ef4444';
  if (value >= 50) return '#f59e0b';
  return '#22c55e';
};

const UsageMeter = ({ utilization, label, detail, note, resetsAt, providerTint, size = 112 }: UsageMeterProps) => {
  const stroke = 8;
  const radius = (size - stroke) / 2;
  const c = 2 * Math.PI * radius;
  const pct = Math.max(0, Math.min(100, utilization));
  const offset = c - (pct / 100) * c;

  return (
    <div className="glass-panel hover-lift" style={{ padding: 10, width: size + 18 }}>
      <svg width={size} height={size} viewBox={`0 0 ${size} ${size}`}>
        <circle cx={size / 2} cy={size / 2} r={radius} fill="none" stroke="var(--meter-track)" strokeWidth={stroke} />
        <circle
          cx={size / 2}
          cy={size / 2}
          r={radius}
          fill="none"
          stroke={arcColor(pct)}
          strokeWidth={stroke}
          strokeLinecap="round"
          strokeDasharray={c}
          strokeDashoffset={offset}
          transform={`rotate(-90 ${size / 2} ${size / 2})`}
          style={{ transition: 'stroke-dashoffset .45s cubic-bezier(.22,.9,.24,1)' }}
        />
      </svg>
      <div style={{ marginTop: -80, textAlign: 'center' }}>
        <div className="metric-value" style={{ fontSize: 22 }}>{pct.toFixed(0)}%</div>
        <div className="metric-label">{label}</div>
      </div>
      <div
        className="metric-label"
        style={{ marginTop: 40, textAlign: 'center', minHeight: 30, lineHeight: 1.3, fontSize: 11 }}
      >
        {detail ?? 'No window detail'}
      </div>
      {note ? (
        <div
          className="metric-label"
          style={{ marginTop: 6, minHeight: 30, textAlign: 'center', lineHeight: 1.25, fontSize: 10 }}
          title={note}
        >
          {note}
        </div>
      ) : null}
      <div style={{ marginTop: 8, textAlign: 'center' }} className={`glass-pill glass-${providerTint}`}>
        <ResetCountdown resetsAt={resetsAt} />
      </div>
    </div>
  );
};

export default UsageMeter;
