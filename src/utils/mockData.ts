import { ModelBreakdownEntry, ProviderId, TrendData, UsageAlert, UsageReport, UsageSnapshot } from '@/types';

const providerOrder: ProviderId[] = ['claude', 'codex', 'gemini'];

const wave = (seed: number, min: number, max: number) => {
  const normalized = (Math.sin(seed) + 1) / 2;
  return min + normalized * (max - min);
};

export const makeMockSnapshot = (provider: ProviderId, now = Date.now()): UsageSnapshot => {
  const idx = providerOrder.indexOf(provider) + 1;
  const sessionPct = wave(now / 600000 + idx, 22, 92);
  const weeklyPct = wave(now / 1800000 + idx * 1.4, 15, 88);
  const sessionLimit = 1000;
  const weeklyLimit = 10000;
  const sessionUsed = Math.round((sessionPct / 100) * sessionLimit);
  const weeklyUsed = Math.round((weeklyPct / 100) * weeklyLimit);

  return {
    provider,
    fetched_at: new Date(now).toISOString(),
    source: 'local_log',
    provenance: 'derived_local',
    stale: false,
    windows: [
      {
        window_type: 'session',
        utilization: sessionPct,
        used: sessionUsed,
        limit: sessionLimit,
        remaining: sessionLimit - sessionUsed,
        reset_countdown_secs: 60 * 60 * 2,
        unit: 'tokens',
        accuracy: 'exact',
      },
      {
        window_type: 'weekly',
        utilization: weeklyPct,
        used: weeklyUsed,
        limit: weeklyLimit,
        remaining: weeklyLimit - weeklyUsed,
        reset_countdown_secs: 60 * 60 * 24 * 3,
        unit: 'tokens',
        accuracy: 'exact',
      },
    ],
    credits: {
      balance_usd: Math.max(0, 120 - idx * 10 - sessionPct),
      spent_usd: 60 + idx * 20 + weeklyPct,
    },
  };
};

export const makeMockTrend = (provider: ProviderId, days = 30, now = Date.now()): TrendData => {
  const idx = providerOrder.indexOf(provider) + 1;
  const points = Array.from({ length: days }, (_, i) => {
    const daySeed = now / 86400000 - (days - i) * 0.45 + idx;
    const cost = wave(daySeed, 0.5, 14);
    const tokens = Math.round(wave(daySeed * 1.17, 8000, 85000));
    const date = new Date(now - (days - i) * 86400000).toISOString().slice(0, 10);

    return {
      date,
      cost_usd: Number(cost.toFixed(2)),
      total_tokens: tokens,
    };
  });

  return {
    provider,
    days,
    points,
    total_cost_usd: Number(points.reduce((sum, p) => sum + p.cost_usd, 0).toFixed(2)),
    total_tokens: points.reduce((sum, p) => sum + p.total_tokens, 0),
  };
};

export const makeMockModelBreakdown = (provider: ProviderId): ModelBreakdownEntry[] => {
  const base: Array<[string, number, number]> = provider === 'claude'
    ? [
        ['claude-sonnet', 140_000, 9.2],
        ['claude-opus', 48_000, 14.8],
      ]
    : provider === 'codex'
      ? [
          ['gpt-5', 180_000, 4.7],
          ['gpt-5-mini', 72_000, 0.9],
        ]
      : [
          ['gemini-2.5-pro', 96_000, 0],
          ['gemini-2.5-flash', 210_000, 0],
        ];

  return base.map(([model, total_tokens, estimated_cost_usd]) => ({
    provider,
    model,
    days: 30,
    input_tokens: Math.round(Number(total_tokens) * 0.62),
    output_tokens: Math.round(Number(total_tokens) * 0.28),
    cache_read_tokens: Math.round(Number(total_tokens) * 0.07),
    cache_write_tokens: Math.round(Number(total_tokens) * 0.03),
    total_tokens: Number(total_tokens),
    estimated_cost_usd: Number(estimated_cost_usd),
  }));
};

export const makeMockAlerts = (snapshots: UsageSnapshot[]): UsageAlert[] =>
  snapshots.flatMap((snapshot) =>
    snapshot.windows
      .filter((window) => (window.utilization ?? 0) >= 75)
      .map((window) => ({
        provider: snapshot.provider,
        window_type: window.window_type,
        utilization: window.utilization,
        threshold_percent: window.utilization >= 95 ? 95 : window.utilization >= 90 ? 90 : 75,
        severity: window.utilization >= 95 ? 'critical' : window.utilization >= 90 ? 'high' : 'warning',
        message: `${snapshot.provider} ${window.window_type} is at ${window.utilization.toFixed(0)}%`,
      }))
  );

export const makeMockUsageReport = (now = Date.now()): UsageReport => {
  const snapshots = providerOrder.map((provider) => makeMockSnapshot(provider, now));
  return {
    generated_at: new Date(now).toISOString(),
    snapshots,
    alerts: makeMockAlerts(snapshots),
    model_breakdowns: providerOrder.flatMap((provider) => makeMockModelBreakdown(provider)),
  };
};
