import { invoke } from '@tauri-apps/api/core';
import { create } from 'zustand';
import { CostEntry, ProviderId, ProviderStatus, RefreshCadence, TrendData, UsageSnapshot } from '@/types';

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

export const useUsageStore = create<UsageState>((set, get) => ({
  snapshots: EMPTY_SNAPSHOTS,
  costHistory: { claude: [], codex: [], gemini: [] },
  trends: { claude: undefined, codex: undefined, gemini: undefined },
  statuses: { claude: undefined, codex: undefined, gemini: undefined },
  loading: false,
  error: undefined,

  fetchSnapshot: async (provider) => {
    const snapshot = await invoke<UsageSnapshot>('get_usage_snapshot', { provider });
    set((state) => ({ snapshots: { ...state.snapshots, [provider]: snapshot } }));
  },

  fetchAll: async () => {
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
    const snapshot = await invoke<UsageSnapshot>('refresh_provider', { provider });
    set((state) => ({ snapshots: { ...state.snapshots, [provider]: snapshot } }));
  },

  refreshAll: async () => {
    const snapshots = await invoke<UsageSnapshot[]>('refresh_all');
    const map = { ...EMPTY_SNAPSHOTS };
    snapshots.forEach((item) => {
      map[item.provider] = item;
    });
    set({ snapshots: map });
  },

  fetchCostHistory: async (provider, days = 30) => {
    const history = await invoke<CostEntry[]>('get_cost_history', { provider, days });
    set((state) => ({ costHistory: { ...state.costHistory, [provider]: history } }));
  },

  fetchTrend: async (provider) => {
    const trend = await invoke<TrendData>('get_usage_trends', { provider });
    set((state) => ({ trends: { ...state.trends, [provider]: trend } }));
  },

  fetchStatus: async (provider) => {
    const status = await invoke<ProviderStatus>('get_provider_status', { provider });
    set((state) => ({ statuses: { ...state.statuses, [provider]: status } }));
  },

  setApiKey: async (provider, key) => {
    await invoke('set_api_key', { provider, key });
    await get().refreshProvider(provider);
  },

  setCadence: async (cadence) => {
    await invoke('set_refresh_cadence', { cadence });
  },

  upsertSnapshot: (snapshot) => {
    set((state) => ({ snapshots: { ...state.snapshots, [snapshot.provider]: snapshot } }));
  },
}));
