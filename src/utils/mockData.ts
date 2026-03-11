import { ProviderId, TrendData, UsageSnapshot } from '@/types';

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
    stale: false,
    windows: [
      {
        window_type: 'session',
        utilization: sessionPct,
        used: sessionUsed,
        limit: sessionLimit,
        remaining: sessionLimit - sessionUsed,
        reset_countdown_secs: 60 * 60 * 2,
      },
      {
        window_type: 'weekly',
        utilization: weeklyPct,
        used: weeklyUsed,
        limit: weeklyLimit,
        remaining: weeklyLimit - weeklyUsed,
        reset_countdown_secs: 60 * 60 * 24 * 3,
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

