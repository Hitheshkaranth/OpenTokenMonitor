import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import { Budget, ProviderId, PerProviderThresholds, RefreshCadence, TrayTitleMode, TrendPreset } from '@/types';

type ThemeMode = 'light' | 'dark' | 'system';

type SettingsState = {
  enabledProviders: Record<ProviderId, boolean>;
  refreshCadence: RefreshCadence;
  apiKeys: Record<ProviderId, string>;
  theme: ThemeMode;
  widgetMode: boolean;
  sidebarCollapsed: boolean;
  launchAtStartup: boolean;
  // Global default window used for trend fetches, alerts, and the comparison
  // surface. A preset (7/30/90) can be overridden with a custom day count.
  trendPreset: TrendPreset;
  trendCustom: boolean;
  customDays: number;
  // A1 notifications opt-in + per-provider utilization alert lines.
  notificationsEnabled: boolean;
  perProviderThresholds: Record<ProviderId, PerProviderThresholds>;
  // A3 budgets + A4 per-provider refresh cadences.
  budgets: Record<ProviderId, Budget>;
  perProviderCadence: Record<ProviderId, RefreshCadence>;
  // Text shown next to the macOS menu-bar icon.
  trayTitleMode: TrayTitleMode;
  // Persist hydration is tracked separately so side effects only run after the
  // user's saved preferences have been loaded from storage.
  hydrated: boolean;
  setProviderEnabled: (provider: ProviderId, enabled: boolean) => void;
  setRefreshCadence: (cadence: RefreshCadence) => void;
  setApiKey: (provider: ProviderId, key: string) => void;
  setTheme: (theme: ThemeMode) => void;
  setWidgetMode: (enabled: boolean) => void;
  setSidebarCollapsed: (collapsed: boolean) => void;
  setLaunchAtStartup: (enabled: boolean) => void;
  setTrendPreset: (preset: TrendPreset) => void;
  setTrendCustom: (custom: boolean) => void;
  setCustomDays: (days: number) => void;
  resolveTrendDays: () => number;
  setNotificationsEnabled: (enabled: boolean) => void;
  setProviderThresholds: (provider: ProviderId, thresholds: PerProviderThresholds) => void;
  setBudgets: (provider: ProviderId, budget: Budget) => void;
  setPerProviderCadence: (provider: ProviderId, cadence: RefreshCadence) => void;
  setTrayTitleMode: (mode: TrayTitleMode) => void;
  markHydrated: () => void;
};

export const useSettingsStore = create<SettingsState>()(
  persist(
    (set, get) => ({
      enabledProviders: { claude: true, codex: true, antigravity: true },
      refreshCadence: 'every2m',
      apiKeys: { claude: '', codex: '', antigravity: '' },
      theme: 'system',
      widgetMode: false,
      sidebarCollapsed: false,
      launchAtStartup: true,
      hydrated: false,
      trendPreset: 30,
      trendCustom: false,
      customDays: 14,
      notificationsEnabled: true,
      perProviderThresholds: {
        claude: { warning: 80, high: 90, critical: 97 },
        codex: { warning: 80, high: 90, critical: 97 },
        antigravity: { warning: 80, high: 90, critical: 97 },
      },
      budgets: {
        claude: { amount_usd: 0, period_days: 30 },
        codex: { amount_usd: 0, period_days: 30 },
        antigravity: { amount_usd: 0, period_days: 30 },
      },
      perProviderCadence: {
        claude: 'every2m',
        codex: 'every2m',
        antigravity: 'every2m',
      },
      trayTitleMode: 'percent',
      setProviderEnabled: (provider, enabled) =>
        set((state) => ({ enabledProviders: { ...state.enabledProviders, [provider]: enabled } })),
      setRefreshCadence: (cadence) => set({ refreshCadence: cadence }),
      setApiKey: (provider, key) => set((state) => ({ apiKeys: { ...state.apiKeys, [provider]: key } })),
      setTheme: (theme) => set({ theme }),
      setWidgetMode: (enabled) => set({ widgetMode: enabled }),
      setSidebarCollapsed: (collapsed) => set({ sidebarCollapsed: collapsed }),
      setLaunchAtStartup: (enabled) => set({ launchAtStartup: enabled }),
      setTrendPreset: (preset) => set({ trendPreset: preset, trendCustom: false }),
      setTrendCustom: (custom) => set({ trendCustom: custom }),
      setCustomDays: (days) =>
        set((state) => ({
          customDays: Math.max(1, Math.min(365, Math.floor(days) || state.customDays)),
          trendCustom: true,
        })),
      resolveTrendDays: () => (get().trendCustom ? get().customDays : get().trendPreset),
      setNotificationsEnabled: (enabled) => set({ notificationsEnabled: enabled }),
      setProviderThresholds: (provider, thresholds) =>
        set((state) => ({
          perProviderThresholds: {
            ...state.perProviderThresholds,
            [provider]: {
              warning: Math.max(0, Math.min(100, thresholds.warning)),
              high: Math.max(0, Math.min(100, thresholds.high)),
              critical: Math.max(0, Math.min(100, thresholds.critical)),
            },
          },
        })),
      setBudgets: (provider, budget) =>
        set((state) => ({
          budgets: {
            ...state.budgets,
            [provider]: {
              amount_usd: Math.max(0, budget.amount_usd),
              period_days: Math.max(1, Math.floor(budget.period_days) || state.budgets[provider].period_days),
            },
          },
        })),
      setPerProviderCadence: (provider, cadence) =>
        set((state) => ({
          perProviderCadence: { ...state.perProviderCadence, [provider]: cadence },
        })),
      setTrayTitleMode: (mode) => set({ trayTitleMode: mode }),
      markHydrated: () => set({ hydrated: true }),
    }),
    {
      name: 'otm-settings-v2',
      // Bump when the persisted shape changes so `migrate` can run against
      // older stored payloads. v1 renamed the `gemini` provider to `antigravity`.
      version: 2,
      // Old payloads keyed provider maps by `gemini`. Carry the user's saved
      // enabled/api-key prefs over to `antigravity` and drop the stale key so a
      // returning user doesn't land on an undefined provider tab.
      migrate: (persisted: unknown, fromVersion: number) => {
        const state = (persisted ?? {}) as Record<string, any>;
        if (fromVersion < 1) {
          for (const key of ['enabledProviders', 'apiKeys'] as const) {
            const map = state[key];
            if (map && typeof map === 'object' && 'gemini' in map) {
              if (!('antigravity' in map)) map.antigravity = map.gemini;
              delete map.gemini;
            }
          }
        }
        return state;
      },
      // Backfill any provider keys missing from persisted maps (e.g. a brand-new
      // provider) so lookups like enabledProviders[id] are never undefined.
      merge: (persisted, current) => {
        const p = (persisted ?? {}) as Partial<SettingsState>;
        return {
          ...current,
          ...p,
          enabledProviders: { ...current.enabledProviders, ...(p.enabledProviders ?? {}) },
          apiKeys: { ...current.apiKeys, ...(p.apiKeys ?? {}) },
          perProviderThresholds: {
            ...current.perProviderThresholds,
            ...(p.perProviderThresholds ?? {}),
          },
          budgets: { ...current.budgets, ...(p.budgets ?? {}) },
          perProviderCadence: {
            ...current.perProviderCadence,
            ...(p.perProviderCadence ?? {}),
          },
        };
      },
      // Only persist user-controlled preferences. Runtime bookkeeping like
      // `hydrated` is intentionally excluded.
      partialize: (state) => ({
        enabledProviders: state.enabledProviders,
        refreshCadence: state.refreshCadence,
        apiKeys: state.apiKeys,
        theme: state.theme,
        widgetMode: state.widgetMode,
        sidebarCollapsed: state.sidebarCollapsed,
        launchAtStartup: state.launchAtStartup,
        trendPreset: state.trendPreset,
        trendCustom: state.trendCustom,
        customDays: state.customDays,
        notificationsEnabled: state.notificationsEnabled,
        perProviderThresholds: state.perProviderThresholds,
        budgets: state.budgets,
        perProviderCadence: state.perProviderCadence,
        trayTitleMode: state.trayTitleMode,
      }),
      // Mark the store as ready once Zustand has merged persisted settings.
      onRehydrateStorage: () => (state) => {
        state?.markHydrated();
      },
    }
  )
);
