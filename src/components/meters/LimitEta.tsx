import ResetCountdown, { formatRemaining } from '@/components/meters/ResetCountdown';
import { UsageWindow, WindowForecast } from '@/types';

type LimitEtaProps = {
  forecast?: WindowForecast;
  // The reset pill class for this surface (pcard-reset, overview-card-reset,
  // widget-provider-reset). The ETA pill reuses it so it inherits the exact
  // same chrome, and only swaps the RESET label for FULL IN.
  className: string;
  // Compact surfaces show the ETA *instead of* the reset pill when the window
  // is on pace to fill before it resets, so cards don't grow.
  replacesReset?: boolean;
  resetsAt?: string;
};

type EtaUrgency = 'soon' | 'warning' | 'critical';

const etaUrgency = (secs: number): EtaUrgency => {
  if (secs <= 15 * 60) return 'critical';
  if (secs <= 60 * 60) return 'warning';
  return 'soon';
};

// Only actionable projections are shown: the window is filling fast enough to
// hit 100% before its reset. Otherwise the reset pill (if any) renders as usual.
export const isLimitImminent = (forecast?: WindowForecast) =>
  Boolean(forecast?.will_hit_before_reset && forecast.eta_to_full_secs != null && forecast.utilization < 100);

export const forecastFor = (forecasts: WindowForecast[] | undefined, window?: UsageWindow) =>
  window ? forecasts?.find((f) => f.window_type === window.window_type) : undefined;

const LimitEta = ({ forecast, className, replacesReset = false, resetsAt }: LimitEtaProps) => {
  const reset = <ResetCountdown resetsAt={resetsAt} className={className} />;
  if (!forecast || !isLimitImminent(forecast)) return replacesReset ? reset : null;

  const eta = forecast.eta_to_full_secs ?? 0;
  const rate = forecast.rate_per_hour != null ? `+${forecast.rate_per_hour.toFixed(1)}%/h over the last 30 min` : '';
  const resets = forecast.resets_in_secs != null ? `resets in ${formatRemaining(forecast.resets_in_secs)}` : '';
  const title = ['On pace to hit the limit before it resets', rate, resets].filter(Boolean).join(' · ');

  return (
    <span className={`${className} limit-eta`} data-urgency={etaUrgency(eta)} title={title}>
      {formatRemaining(eta)}
    </span>
  );
};

export default LimitEta;
