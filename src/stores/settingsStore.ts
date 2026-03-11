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
  demoMode: boolean;
  setProviderEnabled: (provider: ProviderId, enabled: boolean) => void;
  setRefreshCadence: (cadence: RefreshCadence) => void;
  setApiKey: (provider: ProviderId, key: string) => void;
  setTheme: (theme: ThemeMode) => void;
  setWidgetMode: (enabled: boolean) => void;
  setDemoMode: (enabled: boolean) => void;
};

export const useSettingsStore = create<SettingsState>()(
  persist(
    (set) => ({
      enabledProviders: { claude: true, codex: true, gemini: true },
      refreshCadence: 'every1m',
      apiKeys: { claude: '', codex: '', gemini: '' },
      theme: 'system',
      widgetMode: false,
      demoMode: false,
      setProviderEnabled: (provider, enabled) =>
        set((state) => ({ enabledProviders: { ...state.enabledProviders, [provider]: enabled } })),
      setRefreshCadence: (cadence) => set({ refreshCadence: cadence }),
      setApiKey: (provider, key) => set((state) => ({ apiKeys: { ...state.apiKeys, [provider]: key } })),
      setTheme: (theme) => set({ theme }),
      setWidgetMode: (enabled) => set({ widgetMode: enabled }),
      setDemoMode: (enabled) => set({ demoMode: enabled }),
    }),
    {
      name: 'otm-settings-v2',
    }
  )
);
