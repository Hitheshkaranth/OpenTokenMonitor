import React from 'react';
import { openUrl } from '@tauri-apps/plugin-opener';
import { ExternalLink } from 'lucide-react';
import { ExactUsageLimits } from '../types';

interface HomeLimitsProps {
  limits: ExactUsageLimits;
}

const usageUrls: Record<string, string> = {
  anthropic: 'https://claude.ai/settings/usage',
  openai: 'https://chatgpt.com/codex/settings/usage',
  google: 'https://aistudio.google.com/',
};

const PROVIDERS = [
  { key: 'anthropic' as const, label: 'Claude',  color: 'var(--claude-primary)'  },
  { key: 'openai'    as const, label: 'Codex',   color: 'var(--openai-primary)'  },
  { key: 'google'    as const, label: 'Gemini',  color: 'var(--gemini-primary)'  },
];

function barColor(remaining: number, limit: number, providerColor: string): string {
  if (limit <= 0) return providerColor;
  const pct = remaining / limit;
  if (pct <= 0.10) return '#F87171';
  if (pct <= 0.30) return '#FBBF24';
  return providerColor;
}


const HomeLimits: React.FC<HomeLimitsProps> = ({ limits }) => {
  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '8px', flex: 1, minHeight: 0 }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <span style={{ fontSize: '10px', fontWeight: 700, textTransform: 'uppercase', letterSpacing: '0.06em', color: 'var(--text-secondary)' }}>
          Usage Limits
        </span>
        <span style={{ fontSize: '9px', color: 'var(--text-muted)', fontFamily: 'var(--font-mono)' }}>
          {limits.lastSyncAt ? `synced ${formatTime(limits.lastSyncAt)}` : 'sync pending'}
        </span>
      </div>

      <div className="home-rows">
        {PROVIDERS.map(({ key, label, color }) => {
          const data    = limits[key];
          const w4h     = data.fourHour;
          const wk      = data.weekly;
          const color4h = barColor(w4h.remaining, w4h.limit, color);
          const colorWk = barColor(wk.remaining,  wk.limit,  color);

          return (
            <div key={key} className="home-row">
              {/* Provider identity */}
              <div className="home-row__provider">
                <span className="home-row__dot" style={{ background: color }} />
                <span className="home-row__name">{label}</span>
                <button
                  className="home-row__link"
                  onClick={() => { openUrl(usageUrls[key]).catch((e) => console.warn('Could not open URL:', e)); }}
                  title={`View ${label} usage online`}
                  style={{ color }}
                >
                  <ExternalLink size={10} />
                </button>
              </div>

              <span className="home-row__sep" />

              {/* 4-hour window */}
              <div className="home-row__window">
                <span className="home-row__win-label">4H</span>
                <span className="home-row__win-num" style={{ color: color4h }}>
                  {w4h.remaining}/{w4h.limit}
                </span>
                <span className="home-row__win-reset">↻ {formatReset(w4h.resetAt, 'fourHour')}</span>
              </div>

              <span className="home-row__sep" />

              {/* Weekly window */}
              <div className="home-row__window">
                <span className="home-row__win-label">WEEK</span>
                <span className="home-row__win-num" style={{ color: colorWk }}>
                  {wk.remaining}/{wk.limit}
                </span>
                <span className="home-row__win-reset">↻ {formatReset(wk.resetAt, 'weekly')}</span>
              </div>
            </div>
          );
        })}
      </div>

      {renderSyncErrors(limits.syncErrors)}
    </div>
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

function formatTime(ms: number): string {
  return new Date(ms).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' });
}

function formatReset(resetAt: string | undefined, type: 'fourHour' | 'weekly'): string {
  if (!resetAt) return '--';
  const d = new Date(resetAt);
  if (isNaN(d.getTime())) return '--';

  if (type === 'fourHour') {
    return d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', hour12: false });
  }

  // Weekly: show day abbreviation if within 8 days, else "in Xd"
  const diffMs = d.getTime() - Date.now();
  const diffDays = Math.ceil(diffMs / (1000 * 60 * 60 * 24));
  if (diffDays <= 0) return 'now';
  if (diffDays <= 8) {
    return d.toLocaleDateString([], { weekday: 'short' });
  }
  return `in ${diffDays}d`;
}

export default HomeLimits;
