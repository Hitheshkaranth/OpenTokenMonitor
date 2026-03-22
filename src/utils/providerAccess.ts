import { ProviderStatus, UsageSnapshot } from '@/types';

export type ProviderAccessHealth = ProviderStatus['health'] | 'unknown';

export type ProviderAccessState = {
  health: ProviderAccessHealth;
  label: 'Live' | 'Local' | 'Retrying' | 'No access' | 'Checking';
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
  snapshot?: UsageSnapshot
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
    return {
      health: 'active',
      label: 'Local',
      detail: 'Showing usage data derived from local session logs.',
      color: providerAccessColor('active'),
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
