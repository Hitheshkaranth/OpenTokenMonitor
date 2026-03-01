import { invoke } from '@tauri-apps/api/core';
import { fetch } from '@tauri-apps/plugin-http';
import { ExactUsageLimits, ProviderName, UsageAuthConfig, UsageLimitWindow, UsageWindows } from '../types';

const DEFAULT_LIMITS = {
  anthropic: { fourHour: 45, weekly: 550 },
  openai: { fourHour: 60, weekly: 700 },
  google: { fourHour: 50, weekly: 600 },
} as const;

const UNKNOWN_ERRORS: Partial<Record<ProviderName, string>> = {};

type PrimitiveCandidate = {
  path: string;
  numeric?: number;
  text?: string;
};

export const buildLocalLimitFallback = (windows: UsageWindows): ExactUsageLimits => {
  return {
    anthropic: fromLocalWindow('anthropic', windows.anthropic.last_4h, windows.anthropic.last_7d),
    openai: fromLocalWindow('openai', windows.openai.last_4h, windows.openai.last_7d),
    google: fromLocalWindow('google', windows.google.last_4h, windows.google.last_7d),
    lastSyncAt: Date.now(),
    syncErrors: UNKNOWN_ERRORS,
  };
};

const fromLocalWindow = (provider: ProviderName, used4h: number, usedWeek: number) => {
  const limit4h = DEFAULT_LIMITS[provider].fourHour;
  const limitWeek = DEFAULT_LIMITS[provider].weekly;
  return {
    fourHour: {
      used: used4h,
      limit: limit4h,
      remaining: Math.max(0, limit4h - used4h),
      resetAt: nextFourHourResetISO(),
      source: 'local' as const,
    },
    weekly: {
      used: usedWeek,
      limit: limitWeek,
      remaining: Math.max(0, limitWeek - usedWeek),
      resetAt: nextWeeklyResetISO(),
      source: 'local' as const,
    },
  };
};

export const fetchExactUsageLimits = async (
  auth: UsageAuthConfig,
  windows: UsageWindows
): Promise<ExactUsageLimits> => {
  const result = buildLocalLimitFallback(windows);
  const syncErrors: Partial<Record<ProviderName, string>> = {};

  const codexToken = (auth.codexBearerToken || '').trim() || (auth.autoBridge ? await getAutoCodexToken() : '');
  if (codexToken) {
    try {
      const codexPayload = await fetchCodexUsagePayload(codexToken);
      const normalized = extractLimitWindows(codexPayload, 'openai');
      if (!normalized) {
        syncErrors.openai = 'Could not parse Codex usage payload';
      } else {
        result.openai = normalized;
      }
    } catch (e: any) {
      syncErrors.openai = e?.message || 'Codex usage fetch failed';
    }
  } else {
    syncErrors.openai = 'No Codex auth token found';
  }

  const claudeCookie = (auth.claudeCookie || '').trim();
  if (claudeCookie) {
    try {
      const claudePayload = await fetchClaudeUsagePayload(claudeCookie);
      const normalized = extractLimitWindows(claudePayload, 'anthropic');
      if (!normalized) {
        syncErrors.anthropic = 'Could not parse Claude usage payload';
      } else {
        result.anthropic = normalized;
      }
    } catch (e: any) {
      syncErrors.anthropic = e?.message || 'Claude usage fetch failed';
    }
  } else {
    syncErrors.anthropic = 'No Claude cookie configured';
  }

  result.lastSyncAt = Date.now();
  result.syncErrors = syncErrors;
  return result;
};

const fetchCodexUsagePayload = async (token: string): Promise<any> => {
  const headers = {
    Authorization: `Bearer ${token}`,
    Origin: 'https://chatgpt.com',
    Referer: 'https://chatgpt.com/codex/settings/usage',
  };

  const endpoints = [
    'https://chatgpt.com/backend-api/codex/settings/usage',
    'https://chatgpt.com/backend-api/codex/usage',
    'https://chatgpt.com/backend-api/codex/rate_limits',
    'https://chatgpt.com/backend-api/settings/codex/usage',
  ];

  let lastError = 'No Codex usage endpoint worked';
  for (const endpoint of endpoints) {
    try {
      return await fetchJson(endpoint, headers);
    } catch (e: any) {
      lastError = e?.message || `Failed: ${endpoint}`;
    }
  }
  throw new Error(lastError);
};

const fetchClaudeUsagePayload = async (cookie: string): Promise<any> => {
  const baseHeaders = {
    Cookie: cookie,
    Origin: 'https://claude.ai',
    Referer: 'https://claude.ai/settings/usage',
  };

  const jsonEndpoints = [
    'https://claude.ai/api/usage',
    'https://claude.ai/api/settings/usage',
    'https://claude.ai/api/account/usage',
  ];

  for (const endpoint of jsonEndpoints) {
    try {
      return await fetchJson(endpoint, baseHeaders);
    } catch {
      // Try next endpoint.
    }
  }

  const htmlRes = await fetch('https://claude.ai/settings/usage', {
    headers: { ...baseHeaders, Accept: 'text/html' },
  });
  if (!htmlRes.ok) throw new Error(`Claude settings page request failed (${htmlRes.status})`);
  const html = await htmlRes.text();
  const nextData = html.match(/<script[^>]*id="__NEXT_DATA__"[^>]*>([\s\S]*?)<\/script>/i);
  if (!nextData || !nextData[1]) throw new Error('Could not extract Claude page data');
  return JSON.parse(nextData[1]);
};

const fetchJson = async (url: string, extraHeaders: Record<string, string>): Promise<any> => {
  const res = await fetch(url, {
    headers: {
      Accept: 'application/json',
      'User-Agent': 'OpenTokenMonitor/1.0',
      ...extraHeaders,
    },
  });
  if (!res.ok) {
    throw new Error(`${url} -> ${res.status}`);
  }
  return res.json();
};

const getAutoCodexToken = async (): Promise<string> => {
  try {
    return await invoke<string>('get_codex_access_token');
  } catch {
    return '';
  }
};

const extractLimitWindows = (payload: any, provider: ProviderName) => {
  const primitives: PrimitiveCandidate[] = [];
  collectPrimitives(payload, [], primitives);

  const fourHour = deriveWindow(primitives, ['4h', '4_hour', 'four_hour', 'rolling_4h', 'window_4h'], provider, 'fourHour');
  const weekly = deriveWindow(primitives, ['week', 'weekly', '7d', '7_day', 'rolling_7d', 'window_7d'], provider, 'weekly');

  if (!fourHour || !weekly) return null;

  return {
    fourHour,
    weekly,
  };
};

const deriveWindow = (
  items: PrimitiveCandidate[],
  bucketHints: string[],
  provider: ProviderName,
  windowType: 'fourHour' | 'weekly'
): UsageLimitWindow | null => {
  const used = pickNumber(items, bucketHints, ['used', 'consumed', 'current', 'spent']);
  const limit = pickNumber(items, bucketHints, ['limit', 'max', 'quota', 'allowance', 'total']);
  const remaining = pickNumber(items, bucketHints, ['remaining', 'left', 'available']);
  const reset = pickText(items, bucketHints, ['reset', 'resets', 'reset_at', 'resets_at', 'next_reset', 'window_end']);

  let resolvedUsed = used;
  let resolvedLimit = limit;
  let resolvedRemaining = remaining;

  if (resolvedLimit === undefined && resolvedUsed !== undefined && resolvedRemaining !== undefined) {
    resolvedLimit = resolvedUsed + resolvedRemaining;
  }
  if (resolvedUsed === undefined && resolvedLimit !== undefined && resolvedRemaining !== undefined) {
    resolvedUsed = Math.max(0, resolvedLimit - resolvedRemaining);
  }
  if (resolvedRemaining === undefined && resolvedUsed !== undefined && resolvedLimit !== undefined) {
    resolvedRemaining = Math.max(0, resolvedLimit - resolvedUsed);
  }

  if (resolvedUsed === undefined || resolvedLimit === undefined || resolvedRemaining === undefined) {
    return null;
  }

  const fallbackLimit = windowType === 'fourHour' ? DEFAULT_LIMITS[provider].fourHour : DEFAULT_LIMITS[provider].weekly;
  const safeLimit = Math.max(1, resolvedLimit || fallbackLimit);

  return {
    used: Math.max(0, Math.round(resolvedUsed)),
    limit: safeLimit,
    remaining: Math.max(0, Math.round(resolvedRemaining)),
    resetAt: normalizeResetText(reset),
    source: 'remote',
  };
};

const collectPrimitives = (value: any, path: string[], out: PrimitiveCandidate[]) => {
  if (typeof value === 'number' && Number.isFinite(value)) {
    out.push({ path: path.join('.').toLowerCase(), numeric: value });
    return;
  }
  if (typeof value === 'string') {
    out.push({ path: path.join('.').toLowerCase(), text: value });
    return;
  }
  if (Array.isArray(value)) {
    value.forEach((item, index) => collectPrimitives(item, [...path, String(index)], out));
    return;
  }
  if (value && typeof value === 'object') {
    Object.entries(value).forEach(([k, v]) => collectPrimitives(v, [...path, k], out));
  }
};

const pickNumber = (items: PrimitiveCandidate[], bucketHints: string[], keyHints: string[]): number | undefined => {
  const scoped = items.filter((item) => item.numeric !== undefined && bucketHints.some((hint) => item.path.includes(hint)));
  const scopedHit = scoped.find((item) => keyHints.some((hint) => item.path.includes(hint)));
  if (scopedHit?.numeric !== undefined) return Number(scopedHit.numeric);

  const globalHit = items.find((item) => item.numeric !== undefined && keyHints.some((hint) => item.path.includes(hint)));
  if (globalHit?.numeric !== undefined) return Number(globalHit.numeric);

  return undefined;
};

const pickText = (items: PrimitiveCandidate[], bucketHints: string[], keyHints: string[]): string | undefined => {
  const scoped = items.filter((item) => typeof item.text === 'string' && bucketHints.some((hint) => item.path.includes(hint)));
  const scopedHit = scoped.find((item) => keyHints.some((hint) => item.path.includes(hint)));
  if (scopedHit?.text) return scopedHit.text;

  const globalHit = items.find((item) => typeof item.text === 'string' && keyHints.some((hint) => item.path.includes(hint)));
  if (globalHit?.text) return globalHit.text;

  return undefined;
};

const normalizeResetText = (value?: string): string | undefined => {
  if (!value) return undefined;
  const trimmed = value.trim();
  if (!trimmed) return undefined;

  // Already a date string.
  if (!Number.isNaN(Date.parse(trimmed))) return trimmed;

  // Handle unix timestamps represented as text.
  if (/^\d{10,13}$/.test(trimmed)) {
    const num = Number(trimmed);
    const ms = trimmed.length === 10 ? num * 1000 : num;
    return new Date(ms).toISOString();
  }

  return trimmed;
};

const nextFourHourResetISO = (): string => {
  const now = new Date();
  const reset = new Date(now);
  reset.setHours(Math.floor(reset.getHours() / 4) * 4 + 4, 0, 0, 0);
  return reset.toISOString();
};

const nextWeeklyResetISO = (): string => {
  const now = new Date();
  const reset = new Date(now);
  const day = reset.getDay();
  const daysUntilMonday = day === 0 ? 1 : 8 - day;
  reset.setDate(reset.getDate() + daysUntilMonday);
  reset.setHours(0, 0, 0, 0);
  return reset.toISOString();
};
