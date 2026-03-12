import { UsageWindow } from '@/types';

const compactNumber = (value?: number | null) => {
  if (value == null) return '-';
  if (value >= 1_000_000) return `${(value / 1_000_000).toFixed(1)}M`;
  if (value >= 1_000) return `${(value / 1_000).toFixed(1)}K`;
  return String(value);
};

export const windowLabel = (window?: UsageWindow) => {
  switch (window?.window_type) {
    case 'five_hour':
      return '5h Window';
    case 'seven_day':
      return '7d Window';
    case 'daily':
      return 'Daily';
    case 'monthly':
      return 'Monthly';
    case 'weekly':
      return 'Weekly';
    case 'session':
      return 'Session';
    default:
      return 'Window';
  }
};

export const windowValueLabel = (window?: UsageWindow) => {
  if (!window) return 'No data';
  if (window.accuracy === 'percent_only' || window.used == null || window.limit == null) {
    return 'Percent-based window';
  }

  const prefix = window.accuracy === 'approximate' ? '~' : '';
  const base = `${prefix}${compactNumber(window.used)} / ${compactNumber(window.limit)}`;

  switch (window.unit) {
    case 'requests':
      return `${base} requests`;
    case 'tokens':
      return `${base} tokens`;
    default:
      return base;
  }
};

export const countdownLabel = (window?: UsageWindow) => {
  const secs = window?.reset_countdown_secs;
  if (secs == null || secs <= 0) return 'Reset unknown';
  if (secs < 7_200) return `${Math.ceil(secs / 60)}m left`;
  if (secs < 172_800) return `${Math.ceil(secs / 3_600)}h left`;
  return `${Math.ceil(secs / 86_400)}d left`;
};
