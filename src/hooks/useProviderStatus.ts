import { useEffect } from 'react';
import { ProviderId } from '@/types';
import { useUsageStore } from '@/stores/usageStore';

const PROVIDERS: ProviderId[] = ['claude', 'codex', 'gemini'];

// Provider health is polled separately from usage snapshots so the UI can
// distinguish "provider reachable" from "usage snapshot currently unavailable".
export const useProviderStatus = () => {
  const fetchStatus = useUsageStore((s) => s.fetchStatus);

  useEffect(() => {
    const run = () => {
      PROVIDERS.forEach((provider) => {
        fetchStatus(provider).catch(() => undefined);
      });
    };

    run();
    const id = window.setInterval(run, 60_000);
    return () => window.clearInterval(id);
  }, [fetchStatus]);
};
