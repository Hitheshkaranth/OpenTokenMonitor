import React, { useEffect, useMemo, useState } from 'react';
import { CliActivity, GeminiStats, ProviderName } from '../types';

interface UsageChartProps {
  claudeHistory: CliActivity[];
  codexHistory: CliActivity[];
  geminiStats: GeminiStats;
  dailyLimits: Record<ProviderName, number>;
  todayUsage: Record<ProviderName, number>;
}

const C_COLOR = '#D97757';
const OA_COLOR = '#10A37F';
const GM_COLOR = '#4285F4';

const UsageChart: React.FC<UsageChartProps> = ({
  claudeHistory,
  codexHistory,
  geminiStats,
  dailyLimits,
  todayUsage,
}) => {
  const [tick, setTick] = useState(() => Date.now());

  useEffect(() => {
    const timer = setInterval(() => setTick(Date.now()), 5_000);
    return () => clearInterval(timer);
  }, []);

  const now = new Date(tick);
  const liveNow = now.getTime();

  const days = useMemo(() => {
    const out: Date[] = [];
    for (let i = 6; i >= 0; i--) {
      const d = new Date(now);
      d.setDate(d.getDate() - i);
      d.setHours(0, 0, 0, 0);
      out.push(d);
    }
    return out;
  }, [now]);

  const { claudeBuckets, codexBuckets, maxCount, totalToday } = useMemo(() => {
    const cb = days.map((d) => {
      const s = d.getTime();
      const e = s + 86400000;
      return claudeHistory.filter((c) => c.timestamp >= s && c.timestamp < e).length;
    });

    const ob = days.map((d) => {
      const s = d.getTime();
      const e = s + 86400000;
      return codexHistory.filter((c) => c.timestamp >= s && c.timestamp < e).length;
    });

    const mx = Math.max(...days.map((_, i) => cb[i] + ob[i]), 1);
    const todayIdx = 6;
    return {
      claudeBuckets: cb,
      codexBuckets: ob,
      maxCount: mx,
      totalToday: cb[todayIdx] + ob[todayIdx],
    };
  }, [days, claudeHistory, codexHistory]);

  const mergedHistory = useMemo(
    () => [...claudeHistory, ...codexHistory].sort((a, b) => b.timestamp - a.timestamp),
    [claudeHistory, codexHistory]
  );

  const minuteBuckets = useMemo(() => {
    const windowMs = 60 * 60 * 1000;
    const bucketMs = 5 * 60 * 1000;
    const bucketCount = 12;
    const start = liveNow - windowMs;
    const buckets = Array.from({ length: bucketCount }, () => ({ claude: 0, codex: 0 }));

    for (const item of mergedHistory) {
      if (item.timestamp < start || item.timestamp > liveNow) continue;
      const idx = Math.min(bucketCount - 1, Math.max(0, Math.floor((item.timestamp - start) / bucketMs)));
      if (item.provider === 'openai') buckets[idx].codex += 1;
      else buckets[idx].claude += 1;
    }

    return buckets;
  }, [mergedHistory, liveNow]);

  const liveWindow5m = useMemo(
    () => mergedHistory.filter((item) => item.timestamp >= liveNow - 5 * 60 * 1000).length,
    [mergedHistory, liveNow]
  );

  const liveWindow60m = useMemo(
    () => mergedHistory.filter((item) => item.timestamp >= liveNow - 60 * 60 * 1000).length,
    [mergedHistory, liveNow]
  );

  const todayIdx = 6;
  const isToday = (i: number) => i === todayIdx;

  const dayLabel = (d: Date) => {
    if (d.toDateString() === now.toDateString()) return 'Today';
    return d.toLocaleDateString('en-US', { weekday: 'short' });
  };

  const liveIntensity = liveWindow5m > 0 ? 'Active' : 'Idle';
  const liveIntensityColor = liveWindow5m > 0 ? '#34D399' : 'var(--text-muted)';

  const limitRows = [
    { label: 'Claude', provider: 'anthropic' as const, color: C_COLOR, unit: 'msgs' },
    { label: 'Codex', provider: 'openai' as const, color: OA_COLOR, unit: 'cmds' },
    { label: 'Gemini', provider: 'google' as const, color: GM_COLOR, unit: 'sessions' },
  ];

  return (
    <div className="animate-slide-up" style={{ display: 'flex', flexDirection: 'column', gap: '10px', paddingBottom: '4px' }}>
      <div className="trend-live-head">
        <div>
          <div className="trend-live-head__title">Realtime Usage</div>
          <div className="trend-live-head__sub">Event stream updates automatically</div>
        </div>
        <div className="trend-live-head__status" style={{ color: liveIntensityColor }}>
          <span className="trend-live-head__dot" style={{ background: liveIntensityColor }} />
          {liveIntensity}
        </div>
      </div>

      <div className="trend-realtime-card">
        <div className="trend-realtime-bars">
          {minuteBuckets.map((bucket, idx) => {
            const total = bucket.claude + bucket.codex;
            const peak = Math.max(...minuteBuckets.map((b) => b.claude + b.codex), 1);
            const h = Math.max(4, Math.round((total / peak) * 100));
            return (
              <div key={idx} className="trend-realtime-bar">
                <div className="trend-realtime-bar__stack" style={{ height: `${h}%` }}>
                  <div style={{ flex: bucket.claude || 0, background: C_COLOR }} />
                  <div style={{ flex: bucket.codex || 0, background: OA_COLOR }} />
                </div>
              </div>
            );
          })}
        </div>
        <div className="trend-realtime-axis">
          <span>-60m</span>
          <span>Now</span>
        </div>
      </div>

      <div className="trend-limit-grid">
        {limitRows.map((row) => {
          const used = todayUsage[row.provider] ?? 0;
          const limit = Math.max(1, dailyLimits[row.provider] ?? 100);
          const pct = Math.min(used / limit, 1);

          return (
            <div key={row.provider} className="trend-limit-card">
              <div className="trend-limit-card__head">
                <span>{row.label}</span>
                <span style={{ color: row.color }}>{Math.round(pct * 100)}%</span>
              </div>
              <div className="trend-limit-card__line">
                <div style={{ width: `${Math.round(pct * 100)}%`, background: row.color }} />
              </div>
              <div className="trend-limit-card__meta">
                <span>{used.toLocaleString()} / {limit.toLocaleString()}</span>
                <span>{row.unit}</span>
              </div>
            </div>
          );
        })}
      </div>

      <div style={{ display: 'flex', gap: '8px' }}>
        <SummaryPill label="Today" value={totalToday} sub="Claude + Codex commands" color={C_COLOR} />
        <SummaryPill label="Last 60m" value={liveWindow60m} sub="recent commands" color={OA_COLOR} />
        <SummaryPill label="Gemini" value={geminiStats.session_count} sub={`${geminiStats.project_count} projects`} color={GM_COLOR} />
      </div>

      <div className="trend-week-card">
        <div className="trend-week-card__title">Seven Day Trend</div>
        <div className="trend-week-bars">
          {days.map((d, i) => {
            const c = claudeBuckets[i];
            const o = codexBuckets[i];
            const total = c + o;
            const h = Math.max(3, Math.round((total / maxCount) * 100));
            return (
              <div key={i} className="trend-week-col">
                <div className="trend-week-col__stack" style={{ height: `${h}%`, boxShadow: isToday(i) ? '0 0 12px rgba(255,255,255,0.08)' : 'none' }}>
                  <div style={{ flex: c || 0, background: C_COLOR }} />
                  <div style={{ flex: o || 0, background: OA_COLOR }} />
                </div>
                <span style={{ color: isToday(i) ? 'var(--text-primary)' : 'var(--text-muted)' }}>{dayLabel(d)}</span>
              </div>
            );
          })}
        </div>
      </div>

      {geminiStats.projects.length > 0 && (
        <div className="trend-week-card">
          <div className="trend-week-card__title">Gemini Projects</div>
          {geminiStats.projects.slice(0, 4).map((p, i) => (
            <div key={i} style={{ fontSize: '11px', color: 'var(--text-secondary)', padding: '2px 0', display: 'flex', alignItems: 'center', gap: '6px' }}>
              <div style={{ width: '4px', height: '4px', borderRadius: '50%', background: GM_COLOR, flexShrink: 0 }} />
              <span style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{p}</span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
};

const SummaryPill = ({ color, label, value, sub }: { color: string; label: string; value: number; sub: string }) => (
  <div
    style={{
      flex: 1,
      background: 'var(--bg-panel)',
      border: `1px solid ${color}44`,
      borderRadius: '10px',
      padding: '6px 8px',
      boxShadow: `0 0 12px ${color}18`,
    }}
  >
    <div style={{ fontSize: '10px', color, fontWeight: 700, letterSpacing: '0.04em' }}>{label}</div>
    <div style={{ fontSize: '15px', fontWeight: 700, fontFamily: 'var(--font-mono)', color: 'var(--text-primary)', lineHeight: 1.2 }}>
      {value}
    </div>
    <div style={{ fontSize: '9px', color: 'var(--text-muted)' }}>{sub}</div>
  </div>
);

export default UsageChart;
