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
      enabledProviders: { claude: true, codex: true, gemini: true },
      refreshCadence: 'every1m',
      apiKeys: { claude: '', codex: '', gemini: '' },
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
