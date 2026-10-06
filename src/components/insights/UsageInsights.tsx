import { useMemo } from 'react';
import { ModelBreakdownEntry, ProviderId } from '@/types';
import { formatUsd } from '@/utils/format';

type UsageInsightsProps = {
  provider: ProviderId;
  breakdown: ModelBreakdownEntry[];
};

// Segment opacities over the provider accent, largest share first.
const SEGMENT_ALPHA = [0.95, 0.6, 0.38, 0.2];

// Claude reports cache reads separately from input; Codex and Antigravity
// count cached tokens inside input_tokens.
const cacheHitRatio = (provider: ProviderId, entries: ModelBreakdownEntry[]) => {
  const read = entries.reduce((sum, e) => sum + e.cache_read_tokens, 0);
  const denominator =
    provider === 'claude'
      ? entries.reduce((sum, e) => sum + e.input_tokens + e.cache_read_tokens + e.cache_write_tokens, 0)
      : entries.reduce((sum, e) => sum + e.input_tokens, 0);
  return denominator > 0 ? read / denominator : null;
};

const UsageInsights = ({ provider, breakdown }: UsageInsightsProps) => {
  const insights = useMemo(() => {
    const totalCost = breakdown.reduce((sum, e) => sum + e.estimated_cost_usd, 0);
    if (breakdown.length === 0 || totalCost <= 0) return null;

    const sorted = [...breakdown].sort((a, b) => b.estimated_cost_usd - a.estimated_cost_usd);
    const top = sorted.slice(0, 3).map((e) => ({ label: e.model, share: e.estimated_cost_usd / totalCost }));
    const rest = sorted.slice(3).reduce((sum, e) => sum + e.estimated_cost_usd, 0);
    if (rest / totalCost >= 0.005) top.push({ label: 'other', share: rest / totalCost });

    return {
      mix: top,
      cacheHit: cacheHitRatio(provider, breakdown),
      saved: breakdown.reduce((sum, e) => sum + (e.cache_savings_usd ?? 0), 0),
      dominant: top[0].share > 0.6 && top[0].label !== 'other' ? top[0] : null,
    };
  }, [provider, breakdown]);

  if (!insights) return null;

  return (
    <div className="pcard-section-panel">
      <span className="pcard-section-title">Insights</span>
      {(insights.cacheHit != null || insights.saved >= 0.01) && (
        <div className="pcard-cost-pills pcard-insight-pills">
          {insights.cacheHit != null && (
            <span className="pcard-cost-chip" title="Share of input tokens served from the prompt cache">
              Cache hit {Math.round(insights.cacheHit * 100)}%
            </span>
          )}
          {insights.saved >= 0.01 && (
            <span className="pcard-cost-chip" title="Cache reads priced at the full input rate, minus what they actually cost">
              Cache saved {formatUsd(insights.saved)}
            </span>
          )}
        </div>
      )}

      <div className="pcard-mix-bar" role="img" aria-label="Share of spend by model">
        {insights.mix.map((segment, i) => (
          <span
            key={segment.label}
            className="pcard-mix-seg"
            style={{ width: `${segment.share * 100}%`, background: `rgb(var(--widget-accent) / ${SEGMENT_ALPHA[i]})` }}
            title={`${segment.label} · ${Math.round(segment.share * 100)}%`}
          />
        ))}
      </div>
      <div className="pcard-model-list">
        {insights.mix.map((segment, i) => (
          <div key={segment.label} className="pcard-model-row">
            <span className="pcard-mix-dot" style={{ background: `rgb(var(--widget-accent) / ${SEGMENT_ALPHA[i]})` }} />
            <span className="pcard-model-name">{segment.label}</span>
            <span className="pcard-model-cost">{Math.round(segment.share * 100)}%</span>
          </div>
        ))}
      </div>
      {insights.dominant && (
        <span className="pcard-mix-hint">
          {insights.dominant.label} is {Math.round(insights.dominant.share * 100)}% of spend this period
        </span>
      )}
    </div>
  );
};

export default UsageInsights;
