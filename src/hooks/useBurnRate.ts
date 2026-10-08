import { useMemo, useRef } from 'react';
import { UsageSnapshot } from '@/types';
import { displayWindows } from '@/utils/usageWindows';

// The number of token samples kept in the sliding window. Burn is derived from
// the difference between the two most recent samples, so two is enough.
const HISTORY_LIMIT = 2;

// A single sampled burn readout for one provider over one refresh interval.
export interface BurnReadout {
  // Most recent total tokens observed for the provider (may be null if the
  // current snapshot exposes no token counts).
  currentTokens: number | null;
  // True once two usable samples exist, meaning tok/s is meaningful.
  hasSamples: boolean;
  // Tokens consumed per second over the sampling window (null if unknown).
  tokPerSec: number | null;
  // Same value scaled to a per-minute readout for compact displays.
  tokensPerMin: number | null;
  tokensStarted: number | null;
  tokensEnded: number | null;
  // Wall-clock length of the sampling window in seconds.
  windowSeconds: number | null;
  // Whether the last hop in token count went up vs. down (null if unknown).
  rising: boolean | null;
}

// Derive a single "token total" from a snapshot by reading the primary window's
// used token count. Falls back to summing any window that carries an absolute
// `used` value, and returns null when no token counts are exposed at all.
export const snapshotTokenTotal = (snapshot?: UsageSnapshot): number | null => {
  if (!snapshot) return null;
  const [primary] = displayWindows(snapshot);
  if (primary?.used != null && Number.isFinite(primary.used)) return primary.used;
  const total = snapshot.windows.reduce((acc, w) => acc + (Number.isFinite(w.used ?? 0) ? w.used ?? 0 : 0), 0);
  return total > 0 ? total : null;
};

// Tracks successive snapshots for a provider and derives how fast tokens are
// being consumed (tok/s and tokens/min) from the delta between two consecutive
// samples. Each snapshot carries a `fetched_at` timestamp the backend refresh
// stamps, which is what anchors the burn-rate time base.
export const useBurnRate = (snapshot?: UsageSnapshot): BurnReadout => {
  const ring = useRef<UsageSnapshot[]>([]);

  if (snapshot) {
    const history = ring.current;
    const last = history[history.length - 1];
    if (!last || last.fetched_at !== snapshot.fetched_at) {
      history.push(snapshot);
      if (history.length > HISTORY_LIMIT) history.shift();
    }
  }

  const readout = useMemo<BurnReadout>(() => {
    const currentTokens = snapshotTokenTotal(snapshot);
    // `!snapshot` is implied by a null total; it is repeated to narrow the type.
    if (currentTokens == null || !snapshot) {
      return {
        currentTokens: null,
        hasSamples: false,
        tokPerSec: null,
        tokensPerMin: null,
        tokensStarted: null,
        tokensEnded: null,
        windowSeconds: null,
        rising: null,
      };
    }

    const history = ring.current;
    if (history.length < 2) {
      return {
        currentTokens,
        hasSamples: false,
        tokPerSec: null,
        tokensPerMin: null,
        tokensStarted: null,
        tokensEnded: null,
        windowSeconds: null,
        rising: null,
      };
    }

    const older = history[history.length - 2];
    const olderTokens = snapshotTokenTotal(older);
    if (olderTokens == null) {
      return {
        currentTokens,
        hasSamples: false,
        tokPerSec: null,
        tokensPerMin: null,
        tokensStarted: null,
        tokensEnded: null,
        windowSeconds: null,
        rising: null,
      };
    }

    const startMs = new Date(older.fetched_at).getTime();
    const endMs = new Date(snapshot.fetched_at).getTime();
    const windowSeconds = (endMs - startMs) / 1000;
    if (!(windowSeconds > 0)) {
      return {
        currentTokens,
        hasSamples: false,
        tokPerSec: null,
        tokensPerMin: null,
        tokensStarted: olderTokens,
        tokensEnded: currentTokens,
        windowSeconds: null,
        rising: null,
      };
    }

    const delta = currentTokens - olderTokens;
    const tokPerSec = Math.max(0, delta / windowSeconds);

    return {
      currentTokens,
      hasSamples: true,
      tokPerSec,
      tokensPerMin: tokPerSec * 60,
      tokensStarted: olderTokens,
      tokensEnded: currentTokens,
      windowSeconds,
      rising: delta > 0,
    };
  }, [snapshot]);

  return readout;
};