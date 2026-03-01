import React from 'react';
import { openUrl } from '@tauri-apps/plugin-opener';
import { ExternalLink } from 'lucide-react';
import { ExactUsageLimits } from '../types';

interface HomeLimitsProps {
  limits: ExactUsageLimits;
}

const HomeLimits: React.FC<HomeLimitsProps> = ({ limits }) => {
  const usageUrls: Record<string, string> = {
    anthropic: 'https://claude.ai/settings/usage',
    openai: 'https://chatgpt.com/codex/settings/usage',
    google: 'https://aistudio.google.com/',
  };

  const rows = [
    { key: 'anthropic' as const, label: 'Claude', color: 'var(--claude-primary)', data: limits.anthropic },
    { key: 'openai' as const, label: 'Codex', color: 'var(--openai-primary)', data: limits.openai },
    { key: 'google' as const, label: 'Gemini', color: 'var(--gemini-primary)', data: limits.google },
  ];

  return (
    <section className="home-limit-matrix">
      <div className="home-limit-matrix__head">
        <span>Limits Dashboard</span>
        <span>{limits.lastSyncAt ? `Synced ${formatTime(limits.lastSyncAt)}` : 'Sync pending'}</span>
      </div>
      <div className="home-limit-matrix__reset">
        <span>4h reset {formatReset(rows[0].data.fourHour.resetAt)}</span>
        <span>Week reset {formatReset(rows[0].data.weekly.resetAt)}</span>
      </div>

      <div className="home-limit-table">
        <div className="home-limit-table__header">Model</div>
        <div className="home-limit-table__header">4h Remaining</div>
        <div className="home-limit-table__header">Weekly Remaining</div>

        {rows.map((row) => {
          const left4h = row.data.fourHour.remaining;
          const leftWeek = row.data.weekly.remaining;
          return (
            <React.Fragment key={row.key}>
              <div className="home-limit-cell home-limit-cell--model">
                <span className="home-limit-dot" style={{ background: row.color }} />
                <span>{row.label}</span>
                <em className={`source-pill source-pill--${row.data.fourHour.source}`}>{row.data.fourHour.source}</em>
              </div>
              <div className="home-limit-cell">
                <strong style={{ color: row.color }}>{left4h} left</strong>
                <small>used {row.data.fourHour.used} / {row.data.fourHour.limit}</small>
              </div>
              <div className="home-limit-cell" style={{ position: 'relative' }}>
                <strong style={{ color: row.color }}>{leftWeek} left</strong>
                <small>used {row.data.weekly.used} / {row.data.weekly.limit}</small>
                <button
                  onClick={() => openUrl(usageUrls[row.key])}
                  title={`View ${row.label} usage online`}
                  style={{
                    position: 'absolute',
                    top: '4px',
                    right: '4px',
                    background: 'none',
                    border: 'none',
                    cursor: 'pointer',
                    color: row.color,
                    padding: '2px',
                    opacity: 0.7,
                    lineHeight: 1,
                  }}
                >
                  <ExternalLink size={10} />
                </button>
              </div>
            </React.Fragment>
          );
        })}
      </div>

      {renderSyncErrors(limits.syncErrors)}
    </section>
  );
};

function renderSyncErrors(syncErrors?: ExactUsageLimits['syncErrors']) {
  if (!syncErrors) return null;
  const items = Object.entries(syncErrors).filter(([, value]) => !!value);
  if (items.length === 0) return null;
  return (
    <div className="home-sync-errors">
      {items.map(([provider, error]) => (
        <div key={provider}>{provider}: {error}</div>
      ))}
    </div>
  );
}

function formatReset(value?: string): string {
  if (!value) return '--';
  const dt = new Date(value);
  if (Number.isNaN(dt.getTime())) return value;
  return dt.toLocaleString([], { weekday: 'short', hour: '2-digit', minute: '2-digit' });
}

function formatTime(ms: number): string {
  return new Date(ms).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' });
}

export default HomeLimits;
