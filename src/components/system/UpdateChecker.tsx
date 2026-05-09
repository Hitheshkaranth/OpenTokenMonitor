import { useEffect, useState } from 'react';
import { check, type Update, type DownloadEvent } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';

export function UpdateChecker() {
  const [update, setUpdate] = useState<Update | null>(null);
  const [progress, setProgress] = useState<{ downloaded: number; total: number } | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [installing, setInstalling] = useState(false);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const result = await check();
        if (!cancelled && result?.available) {
          setUpdate(result);
        }
      } catch (e: unknown) {
        if (!cancelled) {
          setError(e instanceof Error ? e.message : String(e));
        }
      }
    })();

    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    if (error) {
      // Non-blocking; just log to console for now.
      console.error('updater check failed:', error);
    }
  }, [error]);

  if (!update) return null;

  const startInstall = async () => {
    setInstalling(true);
    setProgress(null);
    try {
      let downloaded = 0;
      await update.downloadAndInstall((event: DownloadEvent) => {
        if (event.event === 'Progress') {
          downloaded += event.data.chunkLength;
          setProgress({
            downloaded,
            total: event.data.contentLength ?? 0,
          });
        }
      });
      await relaunch();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
      setInstalling(false);
    }
  };

  return (
    <div className="update-banner" role="status" aria-live="polite">
      <span>Update {update.version} is available.</span>
      {!installing ? (
        <button type="button" onClick={startInstall}>Install now</button>
      ) : (
        <span>
          {progress
            ? `Downloading… ${Math.round((progress.downloaded / Math.max(progress.total, 1)) * 100)}%`
            : 'Preparing…'}
        </span>
      )}
    </div>
  );
}
