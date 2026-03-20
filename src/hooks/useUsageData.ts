import { listen } from '@tauri-apps/api/event';
import { useEffect } from 'react';
import { ProviderId, UsageSnapshot } from '@/types';
import { useUsageStore } from '@/stores/usageStore';
import { isTauriRuntime } from '@/utils/runtime';

// Bridges the backend event stream into the frontend store. On startup it tries
// to fetch fresh snapshots, then keeps the store in sync with `usage-updated`
// events emitted by the Rust backend.
export const useUsageData = () => {
  const fetchAll = useUsageStore((s) => s.fetchAll);
  const refreshAll = useUsageStore((s) => s.refreshAll);
  const fetchRecentActivity = useUsageStore((s) => s.fetchRecentActivity);
  const upsertSnapshot = useUsageStore((s) => s.upsertSnapshot);
  const providers: ProviderId[] = ['claude', 'codex', 'gemini'];

  useEffect(() => {
    const bootstrap = async () => {
      try {
        // Prefer a backend refresh so the UI starts from current provider state
        // rather than only whatever happened to be persisted locally.
        await refreshAll();
      } catch {
        // If a live refresh fails, cached snapshots still let the app render
        // instead of presenting a blank dashboard.
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
        // `refresh_all` emits a batch; merge each snapshot individually so the
        // store keeps its provider-keyed shape.
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
