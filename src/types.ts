export type ProviderName = 'anthropic' | 'openai' | 'google';

export interface ProviderConfig {
  apiKey: string;
  enabled: boolean;
}

export interface RateLimitInfo {
  requestsLimit: number;
  requestsRemaining: number;
  requestsReset: string;
  tokensLimit: number;
  tokensRemaining: number;
  tokensReset: string;
}

export interface UsageData {
  provider: ProviderName;
  displayName: string;
  icon: string;
  color: string;
  accentColor: string;
  status: 'ok' | 'warning' | 'critical' | 'error' | 'loading';
  error?: string;
  rateLimits?: RateLimitInfo;
  totalTokensUsed?: number;
  totalCost?: number;
  inputTokens?: number;
  outputTokens?: number;
  requestCount?: number;
  lastUpdated: number;
}

export interface UsageSnapshot {
  timestamp: number;
  anthropic?: { tokens: number; cost: number };
  openai?: { tokens: number; cost: number };
  google?: { tokens: number; cost: number };
}

export interface AppConfig {
  providers: Record<ProviderName, ProviderConfig>;
  refreshInterval: number;
  usageHistory: UsageSnapshot[];
  theme: 'dark' | 'light';
  /** Daily command target per provider (used to show X / limit progress). */
  dailyLimits: Record<ProviderName, number>;
  usageAuth?: UsageAuthConfig;
}

export interface CliActivity {
  provider: string;
  command: string;
  timestamp: number;
  project?: string;
}

/** Rich usage data from ~/.claude/stats-cache.json (same source as /usage). */
export interface ClaudeUsageCache {
  total_messages:  number;
  total_sessions:  number;
  messages_today:  number;
  sessions_today:  number;
  tokens_today:    number;
  tokens_total:    number;
}

/** Full summary stats returned by get_claude_stats / get_codex_stats. */
export interface CliSummaryStats {
  total_commands:  number;
  commands_today:  number;
  unique_sessions: number;
  unique_projects: number;
  projects:        string[];
}

export interface GeminiStats {
  project_count: number;
  session_count: number;
  projects: string[];
}

export interface ProviderUsageWindow {
  last_4h: number;
  last_7d: number;
}

export interface UsageWindows {
  anthropic: ProviderUsageWindow;
  openai: ProviderUsageWindow;
  google: ProviderUsageWindow;
}

export interface UsageAuthConfig {
  autoBridge: boolean;
  codexBearerToken: string;
  claudeCookie: string;
}

export interface UsageLimitWindow {
  used: number;
  limit: number;
  remaining: number;
  resetAt?: string;
  source: 'remote' | 'local';
}

export interface ProviderUsageLimits {
  fourHour: UsageLimitWindow;
  weekly: UsageLimitWindow;
}

export interface ExactUsageLimits {
  anthropic: ProviderUsageLimits;
  openai: ProviderUsageLimits;
  google: ProviderUsageLimits;
  lastSyncAt?: number;
  syncErrors?: Partial<Record<ProviderName, string>>;
}
