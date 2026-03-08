export type ProviderId = 'claude' | 'codex' | 'gemini';
export type ProviderTab = ProviderId | 'overview';

export type DataSource = 'oauth' | 'cookie' | 'cli' | 'local_log';

export type WindowType = 'five_hour' | 'seven_day' | 'daily' | 'monthly' | 'session' | 'weekly';

export type RefreshCadence = 'manual' | 'every30s' | 'every1m' | 'every2m' | 'every5m' | 'every15m';

export interface UsageWindow {
  window_type: WindowType;
  utilization: number;
  used?: number;
  limit?: number;
  remaining?: number;
  resets_at?: string;
  reset_countdown_secs?: number;
}

export interface UsageSnapshot {
  provider: ProviderId;
  windows: UsageWindow[];
  credits?: { balance_usd?: number; spent_usd?: number };
  plan?: { tier?: string; note?: string };
  fetched_at: string;
  source: DataSource;
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

export type ProviderHealth = 'active' | 'waiting' | 'error';

export interface ProviderStatus {
  provider: ProviderId;
  health: ProviderHealth;
  message: string;
  checked_at: string;
}
