import { invoke } from '@tauri-apps/api/core';
import { create } from 'zustand';
import {
  CostEntry,
  ModelBreakdownEntry,
  ProviderId,
  ProviderStatus,
  RefreshCadence,
  TrendData,
  UsageAlert,
  UsageReport,
  UsageSnapshot,
} from '@/types';
import { isTauriRuntime } from '@/utils/runtime';

type UsageState = {
  snapshots: Record<ProviderId, UsageSnapshot | undefined>;
  costHistory: Record<ProviderId, CostEntry[]>;
  trends: Record<ProviderId, TrendData | undefined>;
  modelBreakdowns: Record<ProviderId, ModelBreakdownEntry[]>;
  statuses: Record<ProviderId, ProviderStatus | undefined>;
  alerts: Record<ProviderId, UsageAlert[]>;
  latestReport?: UsageReport;
  loading: boolean;
  error?: string;
  fetchSnapshot: (provider: ProviderId) => Promise<void>;
  fetchAll: () => Promise<void>;
  refreshProvider: (provider: ProviderId) => Promise<void>;
  refreshAll: () => Promise<void>;
  fetchCostHistory: (provider: ProviderId, days?: number) => Promise<void>;
  fetchTrend: (provider: ProviderId) => Promise<void>;
  fetchModelBreakdown: (provider: ProviderId, days?: number) => Promise<void>;
  fetchUsageReport: (days?: number) => Promise<void>;
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
  modelBreakdowns: { claude: [], codex: [], gemini: [] },
  statuses: { claude: undefined, codex: undefined, gemini: undefined },
  alerts: { claude: [], codex: [], gemini: [] },
  latestReport: undefined,
  loading: false,
  error: undefined,

  fetchSnapshot: async (provider) => {
    if (!isTauriRuntime()) return;
    const snapshot = await invoke<UsageSnapshot>('get_usage_snapshot', { provider });
    set((state) => ({ snapshots: { ...state.snapshots, [provider]: snapshot } }));
  },

  fetchAll: async () => {
    if (!isTauriRuntime()) return;
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
    if (!isTauriRuntime()) return;
    const snapshot = await invoke<UsageSnapshot>('refresh_provider', { provider });
    set((state) => ({ snapshots: { ...state.snapshots, [provider]: snapshot } }));
  },

  refreshAll: async () => {
    if (!isTauriRuntime()) return;
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
    if (!isTauriRuntime()) return;
    const history = await invoke<CostEntry[]>('get_cost_history', { provider, days });
    set((state) => ({ costHistory: { ...state.costHistory, [provider]: history } }));
  },

  fetchTrend: async (provider) => {
    if (!isTauriRuntime()) return;
    const trend = await invoke<TrendData>('get_usage_trends', { provider });
    set((state) => ({ trends: { ...state.trends, [provider]: trend } }));
  },

  fetchModelBreakdown: async (provider, days = 30) => {
    if (!isTauriRuntime()) return;
    const breakdown = await invoke<ModelBreakdownEntry[]>('get_model_breakdown', { provider, days });
    set((state) => ({ modelBreakdowns: { ...state.modelBreakdowns, [provider]: breakdown } }));
  },

  fetchUsageReport: async (days = 30) => {
    if (!isTauriRuntime()) return;
    const report = await invoke<UsageReport>('export_usage_report', { days });
    set({
      latestReport: report,
      alerts: {
        claude: report.alerts.filter((alert) => alert.provider === 'claude'),
        codex: report.alerts.filter((alert) => alert.provider === 'codex'),
        gemini: report.alerts.filter((alert) => alert.provider === 'gemini'),
      },
      modelBreakdowns: {
        claude: report.model_breakdowns.filter((entry) => entry.provider === 'claude'),
        codex: report.model_breakdowns.filter((entry) => entry.provider === 'codex'),
        gemini: report.model_breakdowns.filter((entry) => entry.provider === 'gemini'),
      },
    });
  },

  fetchStatus: async (provider) => {
    if (!isTauriRuntime()) return;
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
