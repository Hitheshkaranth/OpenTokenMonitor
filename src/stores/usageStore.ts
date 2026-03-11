import { invoke } from '@tauri-apps/api/core';
import { create } from 'zustand';
import { CostEntry, ProviderId, ProviderStatus, RefreshCadence, TrendData, UsageSnapshot } from '@/types';
import { makeMockSnapshot, makeMockTrend } from '@/utils/mockData';
import { isTauriRuntime } from '@/utils/runtime';
import { useSettingsStore } from '@/stores/settingsStore';

type UsageState = {
  snapshots: Record<ProviderId, UsageSnapshot | undefined>;
  costHistory: Record<ProviderId, CostEntry[]>;
  trends: Record<ProviderId, TrendData | undefined>;
  statuses: Record<ProviderId, ProviderStatus | undefined>;
  loading: boolean;
  error?: string;
  fetchSnapshot: (provider: ProviderId) => Promise<void>;
  fetchAll: () => Promise<void>;
  refreshProvider: (provider: ProviderId) => Promise<void>;
  refreshAll: () => Promise<void>;
  fetchCostHistory: (provider: ProviderId, days?: number) => Promise<void>;
  fetchTrend: (provider: ProviderId) => Promise<void>;
  fetchStatus: (provider: ProviderId) => Promise<void>;
  setApiKey: (provider: ProviderId, key: string) => Promise<void>;
  setCadence: (cadence: RefreshCadence) => Promise<void>;
  upsertSnapshot: (snapshot: UsageSnapshot) => void;
};

const EMPTY_SNAPSHOTS: Record<ProviderId, UsageSnapshot | undefined> = {
  claude: undefined,
  codex: undefined,
  gemini: undefined,
};

const PROVIDERS: ProviderId[] = ['claude', 'codex', 'gemini'];

const shouldUseDemoData = () => !isTauriRuntime() || useSettingsStore.getState().demoMode;

const demoSnapshotMap = () => {
  const now = Date.now();
  const map = { ...EMPTY_SNAPSHOTS };
  PROVIDERS.forEach((provider) => {
    map[provider] = makeMockSnapshot(provider, now);
  });
  return map;
};

export const useUsageStore = create<UsageState>((set, get) => ({
  snapshots: EMPTY_SNAPSHOTS,
  costHistory: { claude: [], codex: [], gemini: [] },
  trends: { claude: undefined, codex: undefined, gemini: undefined },
  statuses: { claude: undefined, codex: undefined, gemini: undefined },
  loading: false,
  error: undefined,

  fetchSnapshot: async (provider) => {
    if (shouldUseDemoData()) {
      set((state) => ({ snapshots: { ...state.snapshots, [provider]: makeMockSnapshot(provider) } }));
      return;
    }
    const snapshot = await invoke<UsageSnapshot>('get_usage_snapshot', { provider });
    set((state) => ({ snapshots: { ...state.snapshots, [provider]: snapshot } }));
  },

  fetchAll: async () => {
    if (shouldUseDemoData()) {
      set({ snapshots: demoSnapshotMap(), loading: false, error: undefined });
      return;
    }
    set({ loading: true, error: undefined });
    try {
      const snapshots = await invoke<UsageSnapshot[]>('get_all_snapshots');
      const map = { ...EMPTY_SNAPSHOTS };
      snapshots.forEach((item) => {
        map[item.provider] = item;
      });
      set({ snapshots: map, loading: false });
    } catch (error: unknown) {
      set({ loading: false, error: String(error) });
    }
  },

  refreshProvider: async (provider) => {
    if (shouldUseDemoData()) {
      set((state) => ({ snapshots: { ...state.snapshots, [provider]: makeMockSnapshot(provider) } }));
      return;
    }
    const snapshot = await invoke<UsageSnapshot>('refresh_provider', { provider });
    set((state) => ({ snapshots: { ...state.snapshots, [provider]: snapshot } }));
  },

  refreshAll: async () => {
    if (shouldUseDemoData()) {
      set({ snapshots: demoSnapshotMap(), error: undefined, loading: false });
      return;
    }
    set({ loading: true, error: undefined });
    try {
      const snapshots = await invoke<UsageSnapshot[]>('refresh_all');
      const map = { ...EMPTY_SNAPSHOTS };
      snapshots.forEach((item) => {
        map[item.provider] = item;
      });
      set({ snapshots: map, loading: false });
    } catch (error: unknown) {
      set({ loading: false, error: String(error) });
    }
  },

  fetchCostHistory: async (provider, days = 30) => {
    if (shouldUseDemoData()) {
      const trend = makeMockTrend(provider, days);
      const history: CostEntry[] = trend.points.map((point) => ({
        date: point.date,
        provider,
        model: 'demo-model',
        input_tokens: Math.round(point.total_tokens * 0.6),
        output_tokens: Math.round(point.total_tokens * 0.4),
        cache_read_tokens: Math.round(point.total_tokens * 0.08),
        cache_write_tokens: Math.round(point.total_tokens * 0.03),
        estimated_cost_usd: point.cost_usd,
      }));
      set((state) => ({ costHistory: { ...state.costHistory, [provider]: history } }));
      return;
    }
    const history = await invoke<CostEntry[]>('get_cost_history', { provider, days });
    set((state) => ({ costHistory: { ...state.costHistory, [provider]: history } }));
  },

  fetchTrend: async (provider) => {
    if (shouldUseDemoData()) {
      const trend = makeMockTrend(provider, 30);
      set((state) => ({ trends: { ...state.trends, [provider]: trend } }));
      return;
    }
    const trend = await invoke<TrendData>('get_usage_trends', { provider });
    set((state) => ({ trends: { ...state.trends, [provider]: trend } }));
  },

  fetchStatus: async (provider) => {
    if (shouldUseDemoData()) {
      set((state) => ({
        statuses: {
          ...state.statuses,
          [provider]: {
            provider,
            health: 'active',
            message: 'Demo mode active',
            checked_at: new Date().toISOString(),
          },
        },
      }));
      return;
    }
    const status = await invoke<ProviderStatus>('get_provider_status', { provider });
    set((state) => ({ statuses: { ...state.statuses, [provider]: status } }));
  },

  setApiKey: async (provider, key) => {
    if (!isTauriRuntime()) return;
    await invoke('set_api_key', { provider, key });
    await get().refreshProvider(provider);
  },

  setCadence: async (cadence) => {
    if (!isTauriRuntime()) return;
    await invoke('set_refresh_cadence', { cadence });
  },

  upsertSnapshot: (snapshot) => {
    set((state) => ({ snapshots: { ...state.snapshots, [snapshot.provider]: snapshot } }));
  },
}));
