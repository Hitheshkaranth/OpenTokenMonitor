type UsageBarProps = {
  pct: number;
  label?: string;
  showPct?: boolean;
};

const barClass = (pct: number) => {
  if (pct > 80) return 'bar-red';
  if (pct >= 50) return 'bar-yellow';
  return 'bar-green';
};

const pctColor = (pct: number) => {
  if (pct > 80) return '#ef4444';
  if (pct >= 50) return '#f59e0b';
  return '#22c55e';
};

const UsageBar = ({ pct, label, showPct = true }: UsageBarProps) => {
  const clamped = Math.max(0, Math.min(100, pct));
  return (
    <div className="usage-bar">
      {label && <span className="usage-bar-label">{label}</span>}
      <div className="usage-bar-track">
        <div
          className={`usage-bar-fill ${barClass(clamped)}`}
          style={{ width: `${clamped}%` }}
        />
      </div>
      {showPct && (
        <span className="usage-bar-pct" style={{ color: pctColor(clamped) }}>
          {clamped.toFixed(0)}%
        </span>
      )}
    </div>
  );
};

export default UsageBar;
