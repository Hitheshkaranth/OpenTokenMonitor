/**
 * ProviderCard
 *
 * Displays the usage status for a single AI provider (Claude, OpenAI, Gemini).
 *
 * Display modes (in priority order):
 *  1. rateLimits  — Claude with API key: shows tokens remaining / limit + reset time
 *  2. cliStats    — provider has CLI history: shows daily command progress bar
 *  3. geminiStats — Gemini CLI: shows project/session counts + daily session progress
 *  4. noKey       — no API key and no CLI data: shows a "Setup →" prompt
 */
import React from 'react';
import { UsageData, GeminiStats, ClaudeUsageCache } from '../types';

// Formats large token counts to human-readable strings (e.g. 1.4M, 230K)
const formatTokens = (n: number): string => {
  if (n >= 1_000_000) return (n / 1_000_000).toFixed(1) + 'M';
  if (n >= 1_000)     return (n / 1_000).toFixed(1) + 'K';
  return String(n);
};

// Converts an ISO reset timestamp to a human-readable countdown (e.g. "~4 min")
const formatResetTime = (isoString: string): string => {
  if (!isoString) return '';
  const msLeft = new Date(isoString).getTime() - Date.now();
  if (msLeft <= 0) return 'now';
  const min = Math.ceil(msLeft / 60_000);
  return min < 60 ? `~${min} min` : `~${Math.ceil(min / 60)} hr`;
};

interface CliStats {
  commands:       number;   // all-time total
  commandsToday:  number;
  uniqueSessions: number;
  uniqueProjects: number;
  projects:       string[];
}

interface Props {
  data:          UsageData;
  delay:         string;
  dailyLimit:    number;          // user-configured daily message/command target
  onSettings?:   () => void;
  cliStats?:     CliStats;
  geminiStats?:  GeminiStats;
  claudeCache?:  ClaudeUsageCache;  // rich data from ~/.claude/stats-cache.json
}

const ProviderCard: React.FC<Props> = ({
  data, delay, dailyLimit, onSettings, cliStats, geminiStats, claudeCache,
}) => {
  const noKey = data.status === 'error' && data.error === 'No API key';

  // ── Mode selection ─────────────────────────────────────────────────────────
  // Priority: Claude cache > rate limits > CLI stats > Gemini stats > no-key fallback
  const hasClaudeCache = !!claudeCache && claudeCache.total_messages > 0;
  const hasRateLimits  = !hasClaudeCache && !!data.rateLimits && data.status === 'ok';
  const useCliStats    = !hasClaudeCache && !hasRateLimits && !!cliStats && cliStats.commands > 0;
  const useGeminiStats = !hasClaudeCache && !hasRateLimits && !useCliStats && !!geminiStats && geminiStats.project_count > 0;
  const hasLocalData   = hasClaudeCache || useCliStats || useGeminiStats;

  // ── Visual state ───────────────────────────────────────────────────────────
  const isActive = hasRateLimits || hasLocalData || data.status === 'ok';
  const dotColor = isActive ? '#22C55E' : noKey ? 'var(--text-muted)' : '#EF4444';
  const iconBg     = isActive ? data.color : `${data.color}33`;
  const iconShadow = isActive ? `0 0 14px ${data.color}55` : 'none';
  const iconColor  = isActive ? 'white' : data.color;

  return (
    <div
      className="animate-slide-up glass-card"
      style={{
        display: 'flex', alignItems: 'center', gap: '10px',
        animationDelay: delay,
        boxShadow: `0 0 18px ${data.color}${isActive ? '2E' : '10'}`,
      }}
    >
      {/* Provider icon badge */}
      <div style={{
        width: '32px', height: '32px', borderRadius: '8px',
        background: iconBg, boxShadow: iconShadow,
        display: 'flex', alignItems: 'center', justifyContent: 'center',
        color: iconColor, fontWeight: 800, fontSize: '16px', flexShrink: 0,
      }}>
        {data.icon}
      </div>

      {/* Middle section */}
      <div style={{ flex: 1, minWidth: 0 }}>
        {/* Name + status dot */}
        <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
          <span style={{ fontSize: '13px', fontWeight: 700 }}>{data.displayName}</span>
          <div style={{
            width: '5px', height: '5px', borderRadius: '50%',
            background: dotColor, flexShrink: 0,
          }} />
        </div>

        {/* ── Mode 0: Claude Code stats-cache (real usage from ~/.claude/stats-cache.json) ── */}
        {hasClaudeCache && (() => {
          const cc  = claudeCache!;
          const pct = dailyLimit > 0 ? Math.min(cc.messages_today / dailyLimit, 1) : 0;
          return (
            <>
              <ProgressBar pct={pct} color={data.color} />
              <div style={{ fontSize: '11px', color: 'var(--text-secondary)', marginTop: '2px' }}>
                <Mono color={data.color}>{cc.messages_today}</Mono>
                {' / '}
                <Mono color={data.color}>{dailyLimit}</Mono>
                {' msgs today'}
                {cc.tokens_today > 0 && (
                  <> · <Mono color={data.color}>{formatTokens(cc.tokens_today)}</Mono> tokens</>
                )}
              </div>
              <div style={{ fontSize: '10px', color: 'var(--text-muted)', marginTop: '1px' }}>
                {cc.total_messages.toLocaleString()} total · {cc.total_sessions} sessions
              </div>
            </>
          );
        })()}

        {/* ── Mode 1: Claude API rate limit ── */}
        {hasRateLimits && (() => {
          const rl          = data.rateLimits!;
          const used        = Math.max(0, rl.tokensLimit - rl.tokensRemaining);
          const pct         = rl.tokensLimit > 0 ? used / rl.tokensLimit : 0;
          const resetStr    = formatResetTime(rl.tokensReset);
          return (
            <>
              <ProgressBar pct={pct} color={data.color} />
              <div style={{ fontSize: '11px', color: 'var(--text-secondary)', marginTop: '2px' }}>
                <Mono color={data.color}>{formatTokens(rl.tokensRemaining)}</Mono>
                {' '}left of{' '}
                <Mono color={data.color}>{formatTokens(rl.tokensLimit)}</Mono>
              </div>
              {resetStr && (
                <div style={{ fontSize: '10px', color: 'var(--text-muted)', marginTop: '1px' }}>
                  Resets {resetStr} · via API
                </div>
              )}
            </>
          );
        })()}

        {/* ── Mode 2: CLI daily command progress ── */}
        {useCliStats && (() => {
          const cs  = cliStats!;
          const pct = dailyLimit > 0 ? Math.min(cs.commandsToday / dailyLimit, 1) : 0;
          return (
            <>
              <ProgressBar pct={pct} color={data.color} />
              {/* Today's progress vs daily limit */}
              <div style={{ fontSize: '11px', color: 'var(--text-secondary)', marginTop: '2px' }}>
                <Mono color={data.color}>{cs.commandsToday}</Mono>
                {' / '}
                <Mono color={data.color}>{dailyLimit}</Mono>
                {' cmds today'}
              </div>
              {/* All-time summary row */}
              <div style={{ fontSize: '10px', color: 'var(--text-muted)', marginTop: '1px' }}>
                {cs.commands} total
                {cs.uniqueSessions > 0 && ` · ${cs.uniqueSessions} sessions`}
                {cs.uniqueProjects > 0 && ` · ${cs.uniqueProjects} projects`}
              </div>
            </>
          );
        })()}

        {/* ── Mode 3: Gemini project/session progress ── */}
        {useGeminiStats && (() => {
          const pct = dailyLimit > 0 ? Math.min(geminiStats!.session_count / dailyLimit, 1) : 0;
          return (
            <>
              <ProgressBar pct={pct} color={data.color} />
              <div style={{ fontSize: '11px', color: 'var(--text-secondary)', marginTop: '2px' }}>
                <Mono color={data.color}>{geminiStats!.session_count}</Mono>
                {' / '}
                <Mono color={data.color}>{dailyLimit}</Mono>
                {' sessions · '}
                <Mono color={data.color}>{geminiStats!.project_count}</Mono>
                {' projects'}
              </div>
              {geminiStats!.projects.length > 0 && (
                <div style={{ fontSize: '10px', color: 'var(--text-muted)', marginTop: '1px',
                  whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>
                  {geminiStats!.projects.slice(0, 2).join(', ')}
                </div>
              )}
            </>
          );
        })()}

        {/* ── Mode 4: Not configured ── */}
        {!hasRateLimits && !hasLocalData && noKey && (
          <div style={{ fontSize: '11px', color: 'var(--text-muted)', marginTop: '3px' }}>
            Not configured
          </div>
        )}

        {/* ── API key set but no rate-limit data and no CLI data ── */}
        {!hasRateLimits && !hasLocalData && !noKey && (
          <div style={{ fontSize: '11px', color: 'var(--text-muted)', marginTop: '3px' }}>
            API key verified · no usage data
          </div>
        )}
      </div>

      {/* Right side badge */}
      {hasRateLimits && (
        <div style={{ textAlign: 'right', flexShrink: 0 }}>
          <div style={{ fontSize: '11px', color: 'var(--text-muted)', fontFamily: 'var(--font-mono)' }}>API</div>
          <div style={{ fontSize: '10px', color: 'var(--text-muted)' }}>live</div>
        </div>
      )}

      {!hasRateLimits && hasLocalData && (
        <div style={{ textAlign: 'right', flexShrink: 0 }}>
          <div style={{ fontSize: '11px', color: 'var(--text-muted)', fontFamily: 'var(--font-mono)' }}>CLI</div>
          <div style={{ fontSize: '10px', color: 'var(--text-muted)' }}>local</div>
        </div>
      )}

      {!hasRateLimits && !hasLocalData && noKey && (
        <button
          onClick={onSettings}
          style={{
            fontSize: '11px', color: data.color,
            background: `${data.color}22`,
            border: `1px solid ${data.color}44`,
            borderRadius: '6px', padding: '4px 8px',
            cursor: 'pointer', flexShrink: 0, whiteSpace: 'nowrap',
          }}
        >
          Setup →
        </button>
      )}
    </div>
  );
};

// ── Sub-components ────────────────────────────────────────────────────────────

/** Thin two-tone progress bar (filled / track). */
const ProgressBar = ({ pct, color }: { pct: number; color: string }) => (
  <div style={{
    height: '3px', width: '100%',
    background: 'rgba(255,255,255,0.08)',
    borderRadius: '2px', marginTop: '5px',
    overflow: 'hidden',
  }}>
    <div style={{
      height: '100%',
      width: `${Math.round(pct * 100)}%`,
      background: pct > 0.85 ? '#EF4444' : pct > 0.65 ? '#F59E0B' : color,
      borderRadius: '2px',
      transition: 'width 0.4s ease',
    }} />
  </div>
);

/** Inline monospace span coloured with the provider accent. */
const Mono = ({ color, children }: { color: string; children: React.ReactNode }) => (
  <span style={{ fontFamily: 'var(--font-mono)', color }}>{children}</span>
);

export default ProviderCard;
