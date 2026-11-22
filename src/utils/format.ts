const trimZero = (value: string) => value.replace(/\.0$/, '');

// Compact dollars so large totals fit the small cards: $162.73, $4K, $4.1K, $1.2M.
export const formatUsd = (value: number) => {
  const v = Number.isFinite(value) ? value : 0;
  const abs = Math.abs(v);
  if (abs >= 1_000_000) return `$${trimZero((v / 1_000_000).toFixed(1))}M`;
  if (abs >= 1_000) return `$${trimZero((v / 1_000).toFixed(1))}K`;
  return `$${v.toFixed(2)}`;
};

// UTC calendar-day key ("YYYY-MM-DD"). Backend usage days are bucketed in UTC
// (post_refresh.rs today_cost_usd), so this is the key that matches them.
export const utcDayKey = (date: Date = new Date()) => date.toISOString().slice(0, 10);

// Trend points are sparse (only days with usage), so the last point is the
// most recent *active* day, not today. Look today up by date instead.
export const todayCostUsd = (points: { date: string; cost_usd: number }[] | undefined) =>
  points?.find((p) => p.date === utcDayKey())?.cost_usd ?? 0;

// Trend points only exist for active days. Fill every UTC calendar day from
// the first point through today with zeros so a chart's category axis is
// calendar-true (a 3-week gap no longer looks like one day). Capped at a year.
export const zeroFillDaily = <T extends { date: string; cost_usd: number; total_tokens: number }>(
  points: T[],
  today: string = utcDayKey()
): { date: string; cost_usd: number; total_tokens: number }[] => {
  if (points.length === 0) return [];
  const byDate = new Map(points.map((p) => [p.date, p]));
  const sorted = [...byDate.keys()].sort();
  const start = Date.parse(`${sorted[0]}T00:00:00Z`);
  const end = Math.max(Date.parse(`${today}T00:00:00Z`), Date.parse(`${sorted[sorted.length - 1]}T00:00:00Z`));
  if (!Number.isFinite(start) || !Number.isFinite(end)) return points;
  const DAY = 86_400_000;
  const first = Math.max(start, end - 365 * DAY);
  const out: { date: string; cost_usd: number; total_tokens: number }[] = [];
  for (let t = first; t <= end; t += DAY) {
    const key = new Date(t).toISOString().slice(0, 10);
    const hit = byDate.get(key);
    out.push(hit ? { date: key, cost_usd: hit.cost_usd, total_tokens: hit.total_tokens } : { date: key, cost_usd: 0, total_tokens: 0 });
  }
  return out;
};
