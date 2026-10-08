import { invoke } from '@tauri-apps/api/core';
import { useEffect, useState } from 'react';
import { check, type Update, type DownloadEvent } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';
import { isTauriRuntime } from '@/utils/runtime';

export function UpdateChecker() {
  const [update, setUpdate] = useState<Update | null>(null);
  const [progress, setProgress] = useState<{ downloaded: number; total: number } | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [installing, setInstalling] = useState(false);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      if (!isTauriRuntime()) return;
      try {
        // Builds without a real signing keypair can never install an update, so
        // asking the release endpoint just produces a 404 and a console error on
        // every launch. Skip the round trip entirely.
        const configured = await invoke<boolean>('is_updater_configured');
        if (cancelled || !configured) return;

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
    setError(null);
    try {
      // The total size only arrives on `Started`; `Progress` carries chunk sizes.
      let downloaded = 0;
      let total = 0;
      await update.downloadAndInstall((event: DownloadEvent) => {
        if (event.event === 'Started') {
          total = event.data.contentLength ?? 0;
        } else if (event.event === 'Progress') {
          downloaded += event.data.chunkLength;
          setProgress({ downloaded, total });
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
        <>
          {error && <span>Update failed — try again.</span>}
          <button type="button" onClick={startInstall}>Install now</button>
        </>
      ) : (
        <span>
          {progress && progress.total > 0
            ? `Downloading… ${Math.min(100, Math.round((progress.downloaded / progress.total) * 100))}%`
            : progress
              ? 'Downloading…'
              : 'Preparing…'}
        </span>
      )}
    </div>
  );
}
