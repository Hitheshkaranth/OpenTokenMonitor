import { useEffect } from 'react';
import { PageId, ProviderId } from '@/types';

/**
 * Global keyboard shortcuts for the dashboard:
 *   Ctrl/Cmd + R   refresh everything
 *   Ctrl/Cmd + ,   open settings
 *   Esc            return to overview (ignored while typing in a field)
 *   1 / 2 / 3      jump to Claude / Codex / Antigravity (enabled ones only)
 *   4              jump to Projects
 *
 * Single-character shortcuts are suppressed while an input/textarea/select is
 * focused so typing doesn't navigate the app.
 */
export const useKeyboardShortcuts = (
  setPage: (page: PageId) => void,
  refreshEverything: () => void,
  enabledProviders?: Partial<Record<ProviderId, boolean>>
) => {
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      const tag = (event.target as HTMLElement | null)?.tagName;
      const isInput = tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT';

      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'r') {
        event.preventDefault();
        refreshEverything();
        return;
      }
      if ((event.metaKey || event.ctrlKey) && event.key === ',') {
        event.preventDefault();
        setPage('settings');
        return;
      }
      // Esc in a field dismisses autocomplete etc.; it must not navigate away
      // and drop unsaved input (e.g. a half-pasted API key).
      if (isInput) return;
      if (event.key === 'Escape') {
        setPage('overview');
        return;
      }

      const digitProvider: Record<string, ProviderId> = { '1': 'claude', '2': 'codex', '3': 'antigravity' };
      const target = digitProvider[event.key];
      if (target) {
        if (enabledProviders?.[target] !== false) setPage(target);
        return;
      }
      if (event.key === '4') setPage('projects');
    };

    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [setPage, refreshEverything, enabledProviders]);
};
