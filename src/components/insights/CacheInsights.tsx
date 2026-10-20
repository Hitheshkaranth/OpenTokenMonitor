import { useEffect, useMemo, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { CacheInsights as CacheInsightsData, ProviderId } from '@/types';
import { formatUsd } from '@/utils/format';
import { isTauriRuntime } from '@/utils/runtime';

type CacheInsightsProps = {
  provider: ProviderId;
  // Changes whenever fresh usage lands, so the panel refetches with it.
  refreshKey: unknown;
};

const pct = (ratio: number) => `${Math.round(ratio * 100)}%`;

// Saved per month at hit rate `h`, for one day's prompt volume, taking every
// miss as a cache write — the same assumption as the break-even line.
const monthlySaving = (tokensPerDay: number, h: number, [input, read, write]: [number, number, number]) =>
  ((tokensPerDay * 30) / 1_000_000) * (input - h * read - (1 - h) * write);

const roundSig = (usd: number) => {
  if (usd === 0) return 0;
  const step = 10 ** (Math.floor(Math.log10(Math.abs(usd))) - 1);
  return Math.round(usd / step) * step;
};

// Whole dollars from $10 up: the projection is rounded, so cents would lie.
const savingLabel = (usd: number) => {
  const abs = Math.abs(usd);
  const text = abs >= 10 && abs < 1_000 ? `$${Math.round(abs)}` : formatUsd(abs);
  return usd < 0 ? `−${text}` : text;
};

const CacheInsights = ({ provider, refreshKey }: CacheInsightsProps) => {
  const [data, setData] = useState<CacheInsightsData | null>(null);
  const [view, setView] = useState<'cached' | 'uncached'>('cached');
  const [assumed, setAssumed] = useState<number | null>(null);

  useEffect(() => {
    if (!isTauriRuntime()) return;
    let cancelled = false;
    invoke<CacheInsightsData>('get_cache_insights', { provider })
      .then((next) => !cancelled && setData(next))
      .catch(() => !cancelled && setData(null));
    return () => {
      cancelled = true;
    };
  }, [provider, refreshKey]);

  // The slider starts at the real 7-day hit rate, per provider.
  useEffect(() => setAssumed(null), [provider]);

  const hit = assumed ?? data?.hit_ratio_7d ?? 0.8;

  // At the last 7 days' average daily volume (quiet days included), rounded to
  // two significant figures: a pace, not a promise.
  const projection = useMemo(() => {
    if (!data?.rates_per_m) return null;
    const days = data.daily_prompt_tokens_7d;
    const perDay = days.reduce((sum, t) => sum + t, 0) / Math.max(days.length, 1);
    if (perDay <= 0) return null;
    return roundSig(monthlySaving(perDay, hit, data.rates_per_m));
  }, [data, hit]);

  if (!data || (data.month_cost_usd <= 0 && data.hit_ratio_7d == null)) return null;

  const saved = data.month_uncached_cost_usd - data.month_cost_usd;
  const shown = view === 'cached' ? data.month_cost_usd : data.month_uncached_cost_usd;
  const low = Math.min(data.month_cost_usd, data.month_uncached_cost_usd);
  const high = Math.max(data.month_cost_usd, data.month_uncached_cost_usd);
  const breakEven = data.break_even_hit_ratio;

  return (
    <div className="pcard-section-panel">
      <div className="pcard-section-head">
        <span className="pcard-section-title">Caching</span>
        <div className="pcard-cost-pills pcard-cache-toggle" role="group" aria-label="Show this month with or without caching">
          {(['cached', 'uncached'] as const).map((v) => (
            <button
              key={v}
              type="button"
              className={`pcard-cost-chip pcard-cache-toggle-btn${view === v ? ' is-active' : ''}`}
              aria-pressed={view === v}
              onClick={() => setView(v)}
            >
              {v === 'cached' ? 'Cached' : 'Uncached'}
            </button>
          ))}
        </div>
      </div>

      <div className="pcard-cache-month">
        <span className="pcard-cache-amount">{formatUsd(shown)}</span>
        <span className="pcard-cache-sub">
          this month {view === 'uncached' ? 'without caching' : 'with caching'} · range {formatUsd(low)}–{formatUsd(high)}
        </span>
      </div>

      <div className="pcard-cache-stats">
        <div
          className="pcard-cache-stat"
          title="This month's cost without caching minus what it actually cost, write premium included"
        >
          <span className="pcard-cache-stat-k">{saved >= 0 ? 'Saved this month' : 'Caching cost extra'}</span>
          <span className="pcard-cache-stat-v">{formatUsd(Math.abs(saved))}</span>
        </div>
        <div className="pcard-cache-stat" title="Cache reads over all prompt tokens, last 7 days">
          <span className="pcard-cache-stat-k">7-day hit rate</span>
          <span className="pcard-cache-stat-v">{data.hit_ratio_7d != null ? pct(data.hit_ratio_7d) : '—'}</span>
        </div>
      </div>

      {breakEven != null && (
        <span className="pcard-mix-hint" title="Assumes every cache miss is billed as a cache write">
          {breakEven > 0 ? `Caching wins above ${pct(breakEven)} hit rate` : 'Caching always wins · no write premium'}
        </span>
      )}

      {projection != null && (
        <label className="pcard-cache-slider">
          <span className="pcard-cache-slider-label">Assume {pct(hit)}</span>
          <span className="pcard-cache-track">
            <input
              type="range"
              min={0}
              max={100}
              step={1}
              value={Math.round(hit * 100)}
              onChange={(e) => setAssumed(Number(e.target.value) / 100)}
              aria-label="Assumed cache hit rate"
              style={{ ['--fill' as string]: `${hit * 100}%` }}
            />
            {breakEven != null && breakEven > 0 && (
              <span className="pcard-cache-tick" style={{ left: `${breakEven * 100}%` }} title={`Break-even ${pct(breakEven)}`} />
            )}
          </span>
          <span
            className="pcard-cache-slider-value"
            title="Monthly saving at the last 7 days' average pace and the assumed hit rate, assuming every miss is a cache write"
          >
            ≈ {savingLabel(projection)}/mo
          </span>
        </label>
      )}
    </div>
  );
};

export default CacheInsights;
