import { useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { isTauriRuntime } from '@/utils/runtime';

/**
 * Keeps the OS-level "launch at startup" entry aligned with the persisted
 * setting once the settings store finishes hydrating from local storage.
 *
 * Runs after every settings change so toggling the preference in the UI
 * updates the OS autostart entry without requiring a restart.
 */
export const useLaunchAtStartupSync = (
  launchAtStartup: boolean,
  settingsHydrated: boolean
) => {
  useEffect(() => {
    if (!settingsHydrated || !isTauriRuntime()) return;

    const sync = async () => {
      try {
        const current = await invoke<boolean>('get_launch_at_startup');
        if (current !== launchAtStartup) {
          await invoke<boolean>('set_launch_at_startup', { enabled: launchAtStartup });
        }
      } catch (err) {
        console.error('launch at startup sync failed', err);
      }
    };

    sync().catch(() => undefined);
  }, [launchAtStartup, settingsHydrated]);
};
