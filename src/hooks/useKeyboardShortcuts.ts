import { useEffect } from 'react';
import { PageId } from '@/types';

/**
 * Global keyboard shortcuts for the dashboard:
 *   Ctrl/Cmd + R   refresh everything
 *   Ctrl/Cmd + ,   open settings
 *   Esc            return to overview
 *   1 / 2 / 3      jump to Claude / Codex / Antigravity
 *   4              jump to Projects
 *
 * Single-character shortcuts are suppressed while an input/textarea/select is
 * focused so typing doesn't navigate the app.
 */
export const useKeyboardShortcuts = (
  setPage: (page: PageId) => void,
  refreshEverything: () => void
) => {
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement;
      const isInput =
        target.tagName === 'INPUT' ||
        target.tagName === 'TEXTAREA' ||
        target.tagName === 'SELECT';

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
      if (event.key === 'Escape') {
        setPage('overview');
        return;
      }

      if (isInput) return;
      if (event.key === '1') setPage('claude');
      if (event.key === '2') setPage('codex');
      if (event.key === '3') setPage('antigravity');
      if (event.key === '4') setPage('projects');
    };

    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [setPage, refreshEverything]);
};
