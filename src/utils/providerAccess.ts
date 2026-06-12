import { AuthState, ProviderStatus, UsageSnapshot } from '@/types';

export type ProviderAccessHealth = ProviderStatus['health'] | 'unknown';

export type ProviderAccessState = {
  health: ProviderAccessHealth;
  label:
    | 'Live'
    | 'Local'
    | 'Retrying'
    | 'No access'
    | 'Checking'
    | 'Local — no auth'
    | 'Local — auth expired'
    | 'Local — fetch failed';
  detail: string;
  color: string;
};

export const providerAccessColor = (health: ProviderAccessHealth) => {
  if (health === 'active') return '#34d399';
  if (health === 'error') return '#f87171';
  if (health === 'unknown') return 'var(--text-muted)';
  return '#fbbf24';
};

export const providerAccessDotClass = (health: ProviderAccessHealth) =>
  health === 'unknown' ? 'health-unknown' : `health-${health}`;

// Keep the UI on a small set of human-facing states even though snapshots can come
// from several backends. The goal is to answer one question consistently:
// "Can the app access live provider usage right now?"
export const getProviderAccessState = (
  status?: ProviderStatus,
  snapshot?: UsageSnapshot,
  authState?: AuthState
): ProviderAccessState => {
  if (snapshot?.stale) {
    return {
      health: 'waiting',
      label: 'Retrying',
      detail: 'Live usage fetch failed, so cached data is being shown while the app retries.',
      color: providerAccessColor('waiting'),
    };
  }

  if (snapshot && (snapshot.provenance === 'derived_local' || snapshot.source === 'local_log')) {
    if (authState?.kind === 'none') {
      return {
        health: 'waiting',
        label: 'Local — no auth',
        detail: 'No provider auth was detected; showing usage derived from local session logs.',
        color: providerAccessColor('waiting'),
      };
    }

    if (authState?.kind === 'oauth') {
      const nowUnix = Math.floor(Date.now() / 1000);
      const expired =
        authState.expires_at_unix_secs != null && authState.expires_at_unix_secs <= nowUnix;
      if (expired) {
        return {
          health: 'waiting',
          label: 'Local — auth expired',
          detail: authState.last_error || 'OAuth is expired; click to refresh.',
          color: providerAccessColor('waiting'),
        };
      }
    }

    // Auth handled by a local CLI/server (e.g. Antigravity): a local-log snapshot
    // just means the server isn't running, not that anything failed.
    if (authState?.kind === 'cli') {
      return {
        health: 'waiting',
        label: 'Local',
        detail: authState.last_error || 'Showing local data — start the CLI for live quota.',
        color: providerAccessColor('waiting'),
      };
    }

    return {
      health: 'active',
      label: 'Local — fetch failed',
      detail: authState?.last_error || 'live fetch unavailable, retrying',
      color: providerAccessColor('waiting'),
    };
  }

  if (snapshot) {
    return {
      health: 'active',
      label: 'Live',
      detail: 'Usage data is accessible.',
      color: providerAccessColor('active'),
    };
  }

  if (status?.health === 'active') {
    return {
      health: 'waiting',
      label: 'Retrying',
      detail: 'Provider data was detected, but a fresh usage snapshot is not available yet.',
      color: providerAccessColor('waiting'),
    };
  }

  if (status?.message) {
    return {
      health: 'error',
      label: 'No access',
      detail: status.message,
      color: providerAccessColor('error'),
    };
  }

  return {
    health: 'unknown',
    label: 'Checking',
    detail: 'Checking provider access.',
    color: providerAccessColor('unknown'),
  };
};
