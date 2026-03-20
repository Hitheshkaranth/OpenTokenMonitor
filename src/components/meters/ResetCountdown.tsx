import { useEffect, useMemo, useState } from 'react';

type ResetCountdownProps = {
  resetsAt?: string;
  className?: string;
};

type CountdownUrgency = 'none' | 'normal' | 'soon' | 'warning' | 'critical' | 'expired';

const formatRemaining = (seconds: number): string => {
  if (seconds <= 0) return 'resetting';
  const d = Math.floor(seconds / 86400);
  const h = Math.floor((seconds % 86400) / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  if (d > 0) return `${d}d ${h}h`;
  return `${h}h ${m}m`;
};

const countdownUrgency = (target: number, seconds: number): CountdownUrgency => {
  if (!target) return 'none';
  if (seconds <= 0) return 'expired';
  if (seconds <= 15 * 60) return 'critical';
  if (seconds <= 60 * 60) return 'warning';
  if (seconds <= 6 * 60 * 60) return 'soon';
  return 'normal';
};

const ResetCountdown = ({ resetsAt, className = 'countdown-text' }: ResetCountdownProps) => {
  const target = useMemo(() => (resetsAt ? new Date(resetsAt).getTime() : 0), [resetsAt]);
  const [now, setNow] = useState(() => Date.now());

  useEffect(() => {
    const id = window.setInterval(() => setNow(Date.now()), 1000);
    return () => window.clearInterval(id);
  }, []);

  if (!target) return <span className={className} data-urgency="none">n/a</span>;
  const sec = Math.max(0, Math.floor((target - now) / 1000));
  return (
    <span className={className} data-urgency={countdownUrgency(target, sec)}>
      {formatRemaining(sec)}
    </span>
  );
};

export default ResetCountdown;
