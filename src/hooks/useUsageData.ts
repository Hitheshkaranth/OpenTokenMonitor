import { listen } from '@tauri-apps/api/event';
import { useEffect } from 'react';
import { UsageSnapshot } from '@/types';
import { useUsageStore } from '@/stores/usageStore';
import { useSettingsStore } from '@/stores/settingsStore';
import { isTauriRuntime } from '@/utils/runtime';

export const useUsageData = () => {
  const fetchAll = useUsageStore((s) => s.fetchAll);
  const refreshAll = useUsageStore((s) => s.refreshAll);
  const upsertSnapshot = useUsageStore((s) => s.upsertSnapshot);
  const demoMode = useSettingsStore((s) => s.demoMode);

  useEffect(() => {
    refreshAll().catch(() => fetchAll());

    if (!isTauriRuntime() || demoMode) {
      const timer = window.setInterval(() => {
        refreshAll().catch(() => fetchAll());
      }, 5000);
      return () => window.clearInterval(timer);
    }

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
  }, [fetchAll, refreshAll, upsertSnapshot, demoMode]);
};
