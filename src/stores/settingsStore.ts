import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import { ProviderId, RefreshCadence } from '@/types';

type ThemeMode = 'light' | 'dark' | 'system';

type SettingsState = {
  enabledProviders: Record<ProviderId, boolean>;
  refreshCadence: RefreshCadence;
  apiKeys: Record<ProviderId, string>;
  theme: ThemeMode;
  widgetMode: boolean;
  sidebarCollapsed: boolean;
  launchAtStartup: boolean;
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
  markHydrated: () => void;
};

export const useSettingsStore = create<SettingsState>()(
  persist(
    (set) => ({
      enabledProviders: { claude: true, codex: true, antigravity: true },
      refreshCadence: 'every1m',
      apiKeys: { claude: '', codex: '', antigravity: '' },
      theme: 'system',
      widgetMode: false,
      sidebarCollapsed: false,
      launchAtStartup: true,
      hydrated: false,
      setProviderEnabled: (provider, enabled) =>
        set((state) => ({ enabledProviders: { ...state.enabledProviders, [provider]: enabled } })),
      setRefreshCadence: (cadence) => set({ refreshCadence: cadence }),
      setApiKey: (provider, key) => set((state) => ({ apiKeys: { ...state.apiKeys, [provider]: key } })),
      setTheme: (theme) => set({ theme }),
      setWidgetMode: (enabled) => set({ widgetMode: enabled }),
      setSidebarCollapsed: (collapsed) => set({ sidebarCollapsed: collapsed }),
      setLaunchAtStartup: (enabled) => set({ launchAtStartup: enabled }),
      markHydrated: () => set({ hydrated: true }),
    }),
    {
      name: 'otm-settings-v2',
      // Bump when the persisted shape changes so `migrate` can run against
      // older stored payloads. v1 renamed the `gemini` provider to `antigravity`.
      version: 1,
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
      }),
      // Mark the store as ready once Zustand has merged persisted settings.
      onRehydrateStorage: () => (state) => {
        state?.markHydrated();
      },
    }
  )
);
