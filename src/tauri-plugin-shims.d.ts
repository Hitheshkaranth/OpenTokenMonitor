declare module '@tauri-apps/plugin-updater' {
  export interface DownloadEvent {
    event: 'Started' | 'Progress' | 'Finished';
    data: {
      chunkLength: number;
      contentLength?: number;
    };
  }

  export class Update {
    version: string;
    available: boolean;
    downloadAndInstall(
      onEvent?: (progress: DownloadEvent) => void,
      options?: unknown
    ): Promise<void>;
  }

  export function check(options?: unknown): Promise<Update | null>;
}

declare module '@tauri-apps/plugin-process' {
  export function relaunch(): Promise<void>;
}
