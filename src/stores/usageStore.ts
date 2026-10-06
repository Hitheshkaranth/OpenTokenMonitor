import { invoke } from '@tauri-apps/api/core';
import { openUrl } from '@tauri-apps/plugin-opener';
import { create } from 'zustand';
import {
  AuthState,
  BudgetConfig,
  BudgetForecast,
  CostEntry,
  ExportFormat,
  ModelBreakdownEntry,
  ProjectUsage,
  ProviderId,
  ProviderStatus,
  RecentActivityEntry,
  RefreshCadence,
  TrendData,
  UsageAlert,
  SessionUsage,
  UsageReport,
  UsageSnapshot,
  WindowForecast,
} from '@/types';
import { isTauriRuntime } from '@/utils/runtime';

// This store is the frontend's single bridge to backend usage data. Every action
// here maps a Tauri command into a normalized per-provider state update so the
// React tree can stay focused on rendering.
type UsageState = {
  snapshots: Record<ProviderId, UsageSnapshot | undefined>;
  costHistory: Record<ProviderId, CostEntry[]>;
  trends: Record<ProviderId, TrendData | undefined>;
  modelBreakdowns: Record<ProviderId, ModelBreakdownEntry[]>;
  recentActivity: Record<ProviderId, RecentActivityEntry[]>;
  statuses: Record<ProviderId, ProviderStatus | undefined>;
  alerts: Record<ProviderId, UsageAlert[]>;
  authStates: Record<ProviderId, AuthState | undefined>;
  latestReport?: UsageReport;
  backendBudgets: Record<ProviderId, BudgetConfig | undefined>;
  budgetsForecast: Record<ProviderId, BudgetForecast | undefined>;
  forecasts: Record<ProviderId, WindowForecast[]>;
  projectUsage: ProjectUsage[];
  sessionUsage: SessionUsage[];
  providerProjectUsage: Record<ProviderId, ProjectUsage[]>;
  loading: boolean;
  error?: string;
  fetchSnapshot: (provider: ProviderId) => Promise<void>;
  fetchAll: () => Promise<void>;
  refreshProvider: (provider: ProviderId) => Promise<void>;
  refreshAll: () => Promise<void>;
  fetchCostHistory: (provider: ProviderId, days?: number) => Promise<void>;
  fetchTrend: (provider: ProviderId, days?: number) => Promise<void>;
  fetchModelBreakdown: (provider: ProviderId, days?: number) => Promise<void>;
  fetchRecentActivity: (provider: ProviderId, limit?: number) => Promise<void>;
  fetchUsageReport: (days?: number) => Promise<void>;
  exportReport: (format: ExportFormat, days?: number) => Promise<void>;
  fetchBudgets: () => Promise<void>;
  fetchBudgetForecast: (provider: ProviderId, days?: number) => Promise<void>;
  fetchForecasts: () => Promise<void>;
  fetchProjectUsage: (days: number) => Promise<void>;
  fetchSessionUsage: (days: number, limit?: number) => Promise<void>;
  fetchProviderProjectUsage: (provider: ProviderId, days: number) => Promise<void>;
  fetchStatus: (provider: ProviderId) => Promise<void>;
  fetchAuthState: (provider: ProviderId) => Promise<void>;
  fetchAllAuthStates: () => Promise<void>;
  setApiKey: (provider: ProviderId, key: string) => Promise<void>;
  clearApiKey: (provider: ProviderId) => Promise<void>;
  setCadence: (cadence: RefreshCadence) => Promise<void>;
  upsertSnapshot: (snapshot: UsageSnapshot) => void;
};

const EMPTY_SNAPSHOTS: Record<ProviderId, UsageSnapshot | undefined> = {
  claude: undefined,
  codex: undefined,
  antigravity: undefined,
};

const EMPTY_BACKEND_BUDGETS: Record<ProviderId, BudgetConfig | undefined> = {
  claude: undefined,
  codex: undefined,
  antigravity: undefined,
};

export const useUsageStore = create<UsageState>((set, get) => ({
  snapshots: EMPTY_SNAPSHOTS,
  costHistory: { claude: [], codex: [], antigravity: [] },
  trends: { claude: undefined, codex: undefined, antigravity: undefined },
  modelBreakdowns: { claude: [], codex: [], antigravity: [] },
  recentActivity: { claude: [], codex: [], antigravity: [] },
  statuses: { claude: undefined, codex: undefined, antigravity: undefined },
  alerts: { claude: [], codex: [], antigravity: [] },
  authStates: { claude: undefined, codex: undefined, antigravity: undefined },
  latestReport: undefined,
  backendBudgets: { claude: undefined, codex: undefined, antigravity: undefined },
  budgetsForecast: { claude: undefined, codex: undefined, antigravity: undefined },
  forecasts: { claude: [], codex: [], antigravity: [] },
  projectUsage: [],
  sessionUsage: [],
  providerProjectUsage: { claude: [], codex: [], antigravity: [] },
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
      // The backend returns a list; the UI reads a stable provider-keyed map.
      const map = { ...EMPTY_SNAPSHOTS };
      snapshots.forEach((item) => {
        map[item.provider] = item;
      });
      set({ snapshots: map, loading: false });
    } catch (error: unknown) {
      const message = String(error);
      set({ loading: false, error: message });
      throw error instanceof Error ? error : new Error(message);
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
      // Reset from the empty shape first so disabled/missing providers do not
      // accidentally keep stale snapshots from a previous refresh.
      const map = { ...EMPTY_SNAPSHOTS };
      snapshots.forEach((item) => {
        map[item.provider] = item;
      });
      set({ snapshots: map, loading: false });
    } catch (error: unknown) {
      const message = String(error);
      set({ loading: false, error: message });
      throw error instanceof Error ? error : new Error(message);
    }
  },

  fetchCostHistory: async (provider, days = 30) => {
    if (!isTauriRuntime()) return;
    const history = await invoke<CostEntry[]>('get_cost_history', { provider, days });
    set((state) => ({ costHistory: { ...state.costHistory, [provider]: history } }));
  },

  fetchTrend: async (provider, days = 30) => {
    if (!isTauriRuntime()) return;
    const trend = await invoke<TrendData>('get_usage_trends', { provider, days });
    set((state) => ({ trends: { ...state.trends, [provider]: trend } }));
  },

  fetchModelBreakdown: async (provider, days = 30) => {
    if (!isTauriRuntime()) return;
    const breakdown = await invoke<ModelBreakdownEntry[]>('get_model_breakdown', { provider, days });
    set((state) => ({ modelBreakdowns: { ...state.modelBreakdowns, [provider]: breakdown } }));
  },

  fetchRecentActivity: async (provider, limit = 3) => {
    if (!isTauriRuntime()) return;
    const recent = await invoke<RecentActivityEntry[]>('get_recent_activity', { provider, limit });
    set((state) => ({ recentActivity: { ...state.recentActivity, [provider]: recent } }));
  },

  fetchUsageReport: async (days = 30) => {
    if (!isTauriRuntime()) return;
    const report = await invoke<UsageReport>('export_usage_report', { days });
    set({
      latestReport: report,
      // The report comes back as flat arrays. Group it once here so view
      // components can read provider-specific slices cheaply.
      alerts: {
        claude: report.alerts.filter((alert) => alert.provider === 'claude'),
        codex: report.alerts.filter((alert) => alert.provider === 'codex'),
        antigravity: report.alerts.filter((alert) => alert.provider === 'antigravity'),
      },
      modelBreakdowns: {
        claude: report.model_breakdowns.filter((entry) => entry.provider === 'claude'),
        codex: report.model_breakdowns.filter((entry) => entry.provider === 'codex'),
        antigravity: report.model_breakdowns.filter((entry) => entry.provider === 'antigravity'),
      },
    });
  },

  // Write the report to <app_data>/exports/ and open it in the OS handler so
  // the user can print-to-PDF or hand it off.
  exportReport: async (format, days = 30) => {
    if (!isTauriRuntime()) return;
    try {
      const path = await invoke<string>('export_report', { days, format });
      openUrl(path, '_self');
    } catch (error) {
      console.error('export report failed', error);
    }
  },

  // Load persisted backend budgets so the UI can surface a forecast even
  // before the user has edited any budget locally.
  fetchBudgets: async () => {
    if (!isTauriRuntime()) return;
    try {
      const backendBudgets = await invoke<BudgetConfig[]>('get_budgets');
      const merged = backendBudgets.reduce<Record<ProviderId, BudgetConfig | undefined>>(
        (acc, config) => {
          if (Object.prototype.hasOwnProperty.call(EMPTY_BACKEND_BUDGETS, config.provider)) {
            acc[config.provider] = config;
          }
          return acc;
        },
        { ...EMPTY_BACKEND_BUDGETS }
      );
      set((state) => ({ backendBudgets: { ...state.backendBudgets, ...merged } }));
    } catch (error) {
      // backend store unavailable; keep the frontend-local budgets as source of truth
      void error;
    }
  },

  // (Re)forecast a provider's budget so the UI can render projected spend.
  fetchBudgetForecast: async (provider, days = 30) => {
    if (!isTauriRuntime()) return;
    try {
      const forecast = await invoke<BudgetForecast>('get_forecast', { provider, days });
      set((state) => ({ budgetsForecast: { ...state.budgetsForecast, [provider]: forecast } }));
    } catch (error) {
      // forecast unavailable (no cost history / budget yet); leave prior forecast
      void error;
    }
  },

  // Time-to-limit projections for every window with recorded history.
  fetchForecasts: async () => {
    if (!isTauriRuntime()) return;
    try {
      const all = await invoke<WindowForecast[]>('get_limit_forecasts');
      set({
        forecasts: {
          claude: all.filter((f) => f.provider === 'claude'),
          codex: all.filter((f) => f.provider === 'codex'),
          antigravity: all.filter((f) => f.provider === 'antigravity'),
        },
      });
    } catch (error) {
      // no history yet; keep the previous projections
      void error;
    }
  },

  fetchProjectUsage: async (days) => {
    if (!isTauriRuntime()) return;
    try {
      const projectUsage = await invoke<ProjectUsage[]>('get_project_usage', { days });
      set({ projectUsage });
    } catch (error) {
      void error;
    }
  },

  fetchSessionUsage: async (days, limit = 50) => {
    if (!isTauriRuntime()) return;
    try {
      const sessionUsage = await invoke<SessionUsage[]>('get_session_usage', { days, limit });
      set({ sessionUsage });
    } catch (error) {
      void error;
    }
  },

  // Projects counting only one provider's sessions (provider detail page).
  fetchProviderProjectUsage: async (provider, days) => {
    if (!isTauriRuntime()) return;
    try {
      const projects = await invoke<ProjectUsage[]>('get_project_usage', { days, provider });
      set((state) => ({ providerProjectUsage: { ...state.providerProjectUsage, [provider]: projects } }));
    } catch (error) {
      void error;
    }
  },

  fetchStatus: async (provider) => {
    if (!isTauriRuntime()) return;
    const status = await invoke<ProviderStatus>('get_provider_status', { provider });
    set((state) => ({ statuses: { ...state.statuses, [provider]: status } }));
  },

  fetchAuthState: async (provider) => {
    if (!isTauriRuntime()) return;
    const authState = await invoke<AuthState>('get_auth_state', { provider });
    set((state) => ({ authStates: { ...state.authStates, [provider]: authState } }));
  },

  fetchAllAuthStates: async () => {
    const providers: ProviderId[] = ['claude', 'codex', 'antigravity'];
    await Promise.all(providers.map((provider) => get().fetchAuthState(provider)));
  },

  setApiKey: async (provider, key) => {
    if (!isTauriRuntime()) return;
    await invoke('set_api_key', { provider, key });
    await get().refreshProvider(provider);
  },

  clearApiKey: async (provider) => {
    if (!isTauriRuntime()) return;
    await invoke('clear_api_key', { provider });
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
