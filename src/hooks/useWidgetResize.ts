import { useEffect } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { LogicalSize } from '@tauri-apps/api/dpi';
import { isTauriRuntime } from '@/utils/runtime';

const WIDGET_HEIGHT = 274;
const FULL_HEIGHT = 390;
const FIXED_WIDTH = 360;

/**
 * Resizes the Tauri window to match widget vs. dashboard mode.
 *
 * The window size is otherwise locked (min === max). To animate between
 * modes we briefly relax the constraints, set the new size, then re-lock so
 * the user can't drag the edge afterwards.
 */
export const useWidgetResize = (widgetMode: boolean) => {
  useEffect(() => {
    if (!isTauriRuntime()) return;

    const targetHeight = widgetMode ? WIDGET_HEIGHT : FULL_HEIGHT;
    const currentWindow = getCurrentWindow();

    (async () => {
      try {
        await currentWindow.setResizable(true);
        // Relax constraints so the resize can apply.
        await currentWindow.setSizeConstraints({
          minWidth: FIXED_WIDTH,
          maxWidth: FIXED_WIDTH,
          minHeight: 100,
          maxHeight: 600,
        });
        await currentWindow.setSize(new LogicalSize(FIXED_WIDTH, targetHeight));
        // Re-lock to the new fixed height.
        await currentWindow.setSizeConstraints({
          minWidth: FIXED_WIDTH,
          maxWidth: FIXED_WIDTH,
          minHeight: targetHeight,
          maxHeight: targetHeight,
        });
        await currentWindow.setResizable(false);
      } catch (err) {
        console.error('resize failed', err);
      }
    })();
  }, [widgetMode]);
};
