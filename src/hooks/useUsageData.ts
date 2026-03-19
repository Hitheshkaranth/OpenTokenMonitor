import { listen } from '@tauri-apps/api/event';
import { useEffect } from 'react';
import { ProviderId, UsageSnapshot } from '@/types';
import { useUsageStore } from '@/stores/usageStore';
import { isTauriRuntime } from '@/utils/runtime';

export const useUsageData = () => {
  const fetchAll = useUsageStore((s) => s.fetchAll);
  const refreshAll = useUsageStore((s) => s.refreshAll);
  const fetchRecentActivity = useUsageStore((s) => s.fetchRecentActivity);
  const upsertSnapshot = useUsageStore((s) => s.upsertSnapshot);
  const providers: ProviderId[] = ['claude', 'codex', 'gemini'];

  useEffect(() => {
    const bootstrap = async () => {
      try {
        await refreshAll();
      } catch {
        await fetchAll();
      } finally {
        providers.forEach((provider) => {
          fetchRecentActivity(provider).catch(() => undefined);
        });
      }
    };

    bootstrap().catch(() => undefined);

    if (!isTauriRuntime()) return;

    const unlistenPromise = listen<UsageSnapshot | UsageSnapshot[]>('usage-updated', (event) => {
      const payload = event.payload;
      if (Array.isArray(payload)) {
        payload.forEach((snapshot) => {
          upsertSnapshot(snapshot);
          fetchRecentActivity(snapshot.provider).catch(() => undefined);
        });
        return;
      }
      upsertSnapshot(payload);
      fetchRecentActivity(payload.provider).catch(() => undefined);
    });

    return () => {
      unlistenPromise.then((off) => off());
    };
  }, [fetchAll, fetchRecentActivity, refreshAll, upsertSnapshot]);
};
