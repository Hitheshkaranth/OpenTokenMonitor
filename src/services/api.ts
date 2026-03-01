/**
 * api.ts — Live data fetchers for each AI provider.
 *
 * What each fetch actually does:
 *
 *  Anthropic — Calls the (free) token-count endpoint to get the current
 *              rate-limit headers. Derives "tokens used this window" as
 *              tokensLimit - tokensRemaining, which is a real live number.
 *
 *  OpenAI    — Calls /v1/models as a key-validity check. OpenAI's billing
 *              API requires org-level keys we don't have, so no usage data
 *              is shown — the card falls back to CLI history stats instead.
 *
 *  Google    — Calls /v1beta/models as a key-validity check. Google does not
 *              expose per-key billing data, so the card falls back to Gemini
 *              CLI stats (project / session counts) instead.
 */
import { fetch } from '@tauri-apps/plugin-http';
import { UsageData } from '../types';

// ── Anthropic ─────────────────────────────────────────────────────────────────
export const fetchAnthropicUsage = async (apiKey: string): Promise<UsageData> => {
  const baseData: UsageData = {
    provider:    'anthropic',
    displayName: 'Claude',
    icon:        '◈',
    color:       '#D97757',
    accentColor: 'rgba(217,119,87,0.2)',
    status:      'loading',
    lastUpdated: Date.now(),
  };

  if (!apiKey) return { ...baseData, status: 'error', error: 'No API key' };

  try {
    // This tiny request is free (not billed) but still triggers rate-limit headers,
    // giving us the current token window usage.
    const response = await fetch('https://api.anthropic.com/v1/messages/count_tokens', {
      method: 'POST',
      headers: {
        'Content-Type':      'application/json',
        'x-api-key':         apiKey,
        'anthropic-version': '2023-06-01',
      },
      body: JSON.stringify({
        model:    'claude-3-5-sonnet-20240620',
        messages: [{ role: 'user', content: 'hi' }],
      }),
    });

    if (!response.ok) throw new Error('Invalid API key or request failed');

    const h = response.headers;
    const tokensLimit     = Number(h.get('anthropic-ratelimit-tokens-limit')     || 0);
    const tokensRemaining = Number(h.get('anthropic-ratelimit-tokens-remaining')  || 0);
    // "tokens used this rate-limit window" = limit minus what's left
    const tokensUsed = Math.max(0, tokensLimit - tokensRemaining);

    return {
      ...baseData,
      status: 'ok',
      totalTokensUsed: tokensUsed,
      rateLimits: {
        requestsLimit:     Number(h.get('anthropic-ratelimit-requests-limit')     || 0),
        requestsRemaining: Number(h.get('anthropic-ratelimit-requests-remaining') || 0),
        requestsReset:     h.get('anthropic-ratelimit-requests-reset')            || '',
        tokensLimit,
        tokensRemaining,
        tokensReset:       h.get('anthropic-ratelimit-tokens-reset')              || '',
      },
    };
  } catch (err: any) {
    return { ...baseData, status: 'error', error: err.message };
  }
};

// ── OpenAI ────────────────────────────────────────────────────────────────────
export const fetchOpenAIUsage = async (apiKey: string): Promise<UsageData> => {
  const baseData: UsageData = {
    provider:    'openai',
    displayName: 'OpenAI',
    icon:        '⬡',
    color:       '#10A37F',
    accentColor: 'rgba(16,163,127,0.2)',
    status:      'loading',
    lastUpdated: Date.now(),
  };

  if (!apiKey) return { ...baseData, status: 'error', error: 'No API key' };

  try {
    // Verify the key works. OpenAI's billing/usage API requires org-level keys,
    // so we can't retrieve token usage from here. The Stats card will show
    // Codex CLI history as the primary data source.
    const response = await fetch('https://api.openai.com/v1/models', {
      headers: { Authorization: `Bearer ${apiKey}` },
    });

    if (!response.ok) throw new Error('Invalid API key');

    // Key is valid — no usage numbers available via this endpoint.
    return { ...baseData, status: 'ok' };
  } catch (err: any) {
    return { ...baseData, status: 'error', error: err.message };
  }
};

// ── Google ────────────────────────────────────────────────────────────────────
export const fetchGoogleUsage = async (apiKey: string): Promise<UsageData> => {
  const baseData: UsageData = {
    provider:    'google',
    displayName: 'Gemini',
    icon:        '✦',
    color:       '#4285F4',
    accentColor: 'rgba(66,133,244,0.2)',
    status:      'loading',
    lastUpdated: Date.now(),
  };

  if (!apiKey) return { ...baseData, status: 'error', error: 'No API key' };

  try {
    // Verify the key works. Google does not expose per-key billing data via REST,
    // so the Stats card will show Gemini CLI project/session counts instead.
    const response = await fetch(
      `https://generativelanguage.googleapis.com/v1beta/models?key=${apiKey}`
    );

    if (!response.ok) throw new Error('Invalid API key');

    // Key is valid — no usage numbers available via this endpoint.
    return { ...baseData, status: 'ok' };
  } catch (err: any) {
    return { ...baseData, status: 'error', error: err.message };
  }
};
