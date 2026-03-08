import { listen } from '@tauri-apps/api/event';
import { useEffect } from 'react';
import { UsageSnapshot } from '@/types';
import { useUsageStore } from '@/stores/usageStore';

export const useUsageData = () => {
  const fetchAll = useUsageStore((s) => s.fetchAll);
  const upsertSnapshot = useUsageStore((s) => s.upsertSnapshot);

  useEffect(() => {
    fetchAll();
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
  }, [fetchAll, upsertSnapshot]);
};
