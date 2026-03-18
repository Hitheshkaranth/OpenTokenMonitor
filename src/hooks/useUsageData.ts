import { listen } from '@tauri-apps/api/event';
import { useEffect } from 'react';
import { UsageSnapshot } from '@/types';
import { useUsageStore } from '@/stores/usageStore';
import { isTauriRuntime } from '@/utils/runtime';

export const useUsageData = () => {
  const fetchAll = useUsageStore((s) => s.fetchAll);
  const refreshAll = useUsageStore((s) => s.refreshAll);
  const upsertSnapshot = useUsageStore((s) => s.upsertSnapshot);

  useEffect(() => {
    refreshAll().catch(() => fetchAll());

    if (!isTauriRuntime()) return;

    const unlistenPromise = listen<UsageSnapshot | UsageSnapshot[]>('usage-updated', (event) => {
      const payload = event.payload;
      if (Array.isArray(payload)) {
        payload.forEach((snapshot) => upsertSnapshot(snapshot));
        return;
      }
      upsertSnapshot(payload);
    });

    return () => {
      unlistenPromise.then((off) => off());
    };
  }, [fetchAll, refreshAll, upsertSnapshot]);
};
