export type ProviderId = 'claude' | 'codex' | 'antigravity';
export type ProviderTab = ProviderId | 'overview' | 'projects';
export type PageId = 'overview' | 'projects' | ProviderId | 'comparison' | 'settings';

export type DataSource = 'oauth' | 'cookie' | 'cli' | 'local_log';
export type DataProvenance = 'official' | 'internal' | 'derived_local';

export type WindowType = 'five_hour' | 'seven_day' | 'daily' | 'monthly' | 'session' | 'weekly';
export type UsageUnit = 'tokens' | 'requests' | 'percent' | 'unknown';
export type WindowAccuracy = 'exact' | 'approximate' | 'percent_only';
export type AlertSeverity = 'warning' | 'high' | 'critical';

export type RefreshCadence = 'manual' | 'every30s' | 'every1m' | 'every2m' | 'every5m' | 'every15m';

// Preset windows offered by the trend-period selector. `custom` is handled by
// the store as a free-form `customDays` number rather than a preset value.
export type TrendPreset = 7 | 30 | 90;

// Per-provider utilization alert lines (percent) backing the A1 notifications.
export interface PerProviderThresholds {
  warning: number;
  high: number;
  critical: number;
}

// File format for the on-disk export report. Serialized lowercase to match the
// backend ExportFormat enum ("csv" | "json" | "pdf").
export type ExportFormat = 'csv' | 'json' | 'pdf';

// Per-provider spend cap backing the A3 budgets.
export interface Budget {
  amount_usd: number;
  period_days: number;
}

// Backend budget entry returned by get_budgets; carries its provider so the
// frontend can key it the same way it tracks budgets locally.
export interface BudgetConfig {
  provider: ProviderId;
  amount_usd: number;
  period_days: number;
}

// Backend forecast for a provider's budget over a period (get_forecast).
export interface BudgetForecast {
  provider: ProviderId;
  period_days: number;
  spend_to_date_usd: number;
  budget_usd: number | null;
  remaining_usd: number | null;
  daily_average_usd: number;
  projected_spend_usd: number;
  projected_exceeds_budget: boolean;
  utilization_percent: number;
}

// What the macOS menu-bar title shows next to the tray icon.
export type TrayTitleMode = 'off' | 'percent' | 'cost';

// Backend time-to-limit projection for one provider window (get_limit_forecasts).
export interface WindowForecast {
  provider: ProviderId;
  window_type: WindowType;
  utilization: number;
  // Percentage points per hour over the last 30 minutes.
  rate_per_hour: number | null;
  eta_to_full_secs: number | null;
  resets_in_secs: number | null;
  will_hit_before_reset: boolean;
  sample_count: number;
}

export interface ModelCost {
  model: string;
  cost_usd: number;
}

// Exact usage of one CLI session (one log file) in the selected period
// (get_session_usage). Claude + Codex only. Timestamps are unix seconds.
export interface SessionUsage {
  provider: ProviderId;
  session_id: string;
  cwd: string | null;
  first_ts: number | null;
  last_ts: number | null;
  input_tokens: number;
  output_tokens: number;
  cache_read_tokens: number;
  cache_write_tokens: number;
  total_tokens: number;
  cost_usd: number;
  models: ModelCost[];
  today_cost_usd: number;
}

// Sessions rolled up by working directory (get_project_usage).
export interface ProjectUsage {
  project_id: string;
  label: string;
  path: string | null;
  providers: ProviderId[];
  session_count: number;
  input_tokens: number;
  output_tokens: number;
  cache_read_tokens: number;
  cache_write_tokens: number;
  total_tokens: number;
  cost_usd: number;
  today_cost_usd: number;
  last_ts: number | null;
  models: ModelCost[];
  cache_hit_ratio: number;
}

export interface UsageWindow {
  window_type: WindowType;
  utilization: number;
  used?: number;
  limit?: number;
  remaining?: number;
  resets_at?: string;
  reset_countdown_secs?: number;
  unit?: UsageUnit;
  accuracy?: WindowAccuracy;
  note?: string;
}

export interface UsageSnapshot {
  provider: ProviderId;
  windows: UsageWindow[];
  credits?: { balance_usd?: number; spent_usd?: number };
  plan?: { tier?: string; note?: string };
  fetched_at: string;
  source: DataSource;
  provenance?: DataProvenance;
  stale: boolean;
}

export interface CostEntry {
  date: string;
  provider: ProviderId;
  model: string;
  input_tokens: number;
  output_tokens: number;
  cache_read_tokens: number;
  cache_write_tokens: number;
  estimated_cost_usd: number;
}

export interface TrendPoint {
  date: string;
  cost_usd: number;
  total_tokens: number;
}

export interface TrendData {
  provider: ProviderId;
  days: number;
  points: TrendPoint[];
  total_cost_usd: number;
  total_tokens: number;
}

export interface ModelBreakdownEntry {
  provider: ProviderId;
  model: string;
  days: number;
  input_tokens: number;
  output_tokens: number;
  cache_read_tokens: number;
  cache_write_tokens: number;
  total_tokens: number;
  estimated_cost_usd: number;
  // Saved by prompt caching versus the full input rate.
  cache_savings_usd?: number;
}

export interface RecentActivityEntry {
  provider: ProviderId;
  prompt: string;
  response?: string;
  timestamp: string;
  session_id?: string;
  terminal_label?: string;
  cwd?: string;
  model?: string;
}

export interface ProjectCommandEntry extends RecentActivityEntry {
  project_id: string;
  project_label: string;
  project_path?: string;
}

export interface ProjectSummary {
  id: string;
  label: string;
  path?: string;
  latest_timestamp: string;
  activity_count: number;
  providers: ProviderId[];
  models: string[];
  estimated_cost_usd: number;
  estimated_cost_today_usd: number;
  estimated_tokens: number;
  commands: ProjectCommandEntry[];
}

export interface UsageAlert {
  provider: ProviderId;
  window_type: WindowType;
  utilization: number;
  threshold_percent: number;
  severity: AlertSeverity;
  message: string;
}

export interface UsageReport {
  generated_at: string;
  snapshots: UsageSnapshot[];
  alerts: UsageAlert[];
  model_breakdowns: ModelBreakdownEntry[];
}

export type ProviderHealth = 'active' | 'waiting' | 'error';
export type AuthKind = 'oauth' | 'api_key' | 'cookie' | 'cli' | 'none';

export type AuthState = {
  provider: ProviderId;
  kind: AuthKind;
  source_path: string;
  expires_at_unix_secs: number | null;
  last_refresh_iso: string | null;
  has_refresh_token: boolean;
  last_error: string | null;
};

export interface ProviderStatus {
  provider: ProviderId;
  health: ProviderHealth;
  message: string;
  checked_at: string;
}
