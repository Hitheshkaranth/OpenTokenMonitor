import { PropsWithChildren } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { BarChart3, Home, Settings2 } from 'lucide-react';
import GlassPill from '@/components/glass/GlassPill';
import ProviderLogo from '@/components/providers/ProviderLogo';
import { ProviderTab } from '@/types';

type AppShellProps = PropsWithChildren<{
  activeTab: ProviderTab;
  onTabChange: (tab: ProviderTab) => void;
  onOpenSettings: () => void;
  onOpenTrends: () => void;
  settingsOpen: boolean;
}>;

const AppShell = ({ children, activeTab, onTabChange, onOpenSettings, onOpenTrends, settingsOpen }: AppShellProps) => {
  const selectedSegment: 'overview' | 'trends' | 'filters' = settingsOpen
    ? 'filters'
    : activeTab === 'overview'
      ? 'overview'
      : 'trends';

  const onToggleMaximize = async () => {
    const win = getCurrentWindow();
    if (await win.isMaximized()) {
      await win.unmaximize();
    } else {
      await win.maximize();
    }
  };

  return (
    <div className="glass-panel" style={{ height: '100%', padding: 10, paddingTop: 14, display: 'grid', gridTemplateRows: 'auto auto auto 1fr', gap: 10 }}>
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginTop: 4 }}>
        <div data-tauri-drag-region style={{ display: 'flex', alignItems: 'center', gap: 10, flex: 1, minWidth: 0 }}>
          <img
            src="/open_token_monitor_icon.png"
            alt="OpenTokenMonitor logo"
            style={{ width: 18, height: 18, objectFit: 'contain' }}
          />
          <div style={{ fontWeight: 800, fontSize: 13, color: 'var(--text-secondary)' }}>OpenToken Monitor</div>
        </div>
        <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
          <button
            type="button"
            aria-label="Minimize"
            title="Minimize"
            onClick={() => getCurrentWindow().hide()}
            style={{ width: 12, height: 12, borderRadius: 999, border: 0, background: '#febc2e', cursor: 'pointer' }}
          />
          <button
            type="button"
            aria-label="Maximize"
            title="Maximize"
            onClick={onToggleMaximize}
            style={{ width: 12, height: 12, borderRadius: 999, border: 0, background: '#28c840', cursor: 'pointer' }}
          />
          <button
            type="button"
            aria-label="Close"
            title="Close"
            onClick={() => invoke('quit_app')}
            style={{ width: 12, height: 12, borderRadius: 999, border: 0, background: '#ff5f57', cursor: 'pointer' }}
          />
        </div>
      </div>

      <div style={{ marginTop: 2 }}>
        <div className="segment-bar">
          <button
            className={`segment-btn ${selectedSegment === 'overview' ? 'segment-btn-active' : ''}`.trim()}
            onClick={() => onTabChange('overview')}
            type="button"
          >
            {selectedSegment === 'overview' && <span className="segment-led" />}
            <Home size={13} />
            Home
          </button>
          <button
            className={`segment-btn ${selectedSegment === 'trends' ? 'segment-btn-active' : ''}`.trim()}
            onClick={onOpenTrends}
            type="button"
          >
            {selectedSegment === 'trends' && <span className="segment-led" />}
            <BarChart3 size={13} />
            Trends
          </button>
          <button
            className={`segment-btn ${selectedSegment === 'filters' ? 'segment-btn-active' : ''}`.trim()}
            onClick={onOpenSettings}
            type="button"
          >
            {selectedSegment === 'filters' && <span className="segment-led" />}
            <Settings2 size={13} />
            <span>Settings</span>
          </button>
        </div>
      </div>

      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, minmax(0, 1fr))', gap: 8 }}>
        <GlassPill active={activeTab === 'claude'} onClick={() => onTabChange('claude')} className="provider-tab-pill">
          <ProviderLogo provider="claude" /> Claude
        </GlassPill>
        <GlassPill active={activeTab === 'codex'} onClick={() => onTabChange('codex')} className="provider-tab-pill">
          <ProviderLogo provider="codex" /> Codex
        </GlassPill>
        <GlassPill active={activeTab === 'gemini'} onClick={() => onTabChange('gemini')} className="provider-tab-pill">
          <ProviderLogo provider="gemini" /> Gemini
        </GlassPill>
      </div>
      <div className="soft-scroll" style={{ overflow: 'auto' }}>{children}</div>
    </div>
  );
};

export default AppShell;
