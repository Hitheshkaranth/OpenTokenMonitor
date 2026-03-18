import { RefreshCw, Home, Settings2 } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import ProviderLogo from '@/components/providers/ProviderLogo';
import { PageId, ProviderId } from '@/types';
import { useSettingsStore } from '@/stores/settingsStore';
import { useUsageStore } from '@/stores/usageStore';
import { isTauriRuntime } from '@/utils/runtime';

type NavBarProps = {
  activePage: PageId;
  onNavigate: (page: PageId) => void;
  onRefresh: () => void;
  refreshBusy: boolean;
  onWidget: () => void;
};

const providers: { id: ProviderId; label: string; tint: string }[] = [
  { id: 'claude', label: 'Claude', tint: 'claude' },
  { id: 'codex', label: 'Codex', tint: 'codex' },
  { id: 'gemini', label: 'Gemini', tint: 'gemini' },
];

const NavBar = ({ activePage, onNavigate, onRefresh, refreshBusy, onWidget }: NavBarProps) => {
  const enabledProviders = useSettingsStore((s) => s.enabledProviders);
  const statuses = useUsageStore((s) => s.statuses);

  return (
    <div className="nav-bar" data-tauri-drag-region>
      {/* Header row: brand + controls */}
      <div className="nav-header" data-tauri-drag-region>
        <div className="nav-brand" data-tauri-drag-region>
          <img src="/open_token_monitor_icon.png" alt="OTM" className="nav-logo" />
          <span className="nav-title">OpenToken Monitor</span>
        </div>
        <div className="nav-controls">
          <button
            className="glass-pill compact-action-btn"
            onClick={onRefresh}
            disabled={refreshBusy}
            title="Refresh all (Ctrl+R)"
            style={{ width: 26, height: 26, minWidth: 26 }}
          >
            <RefreshCw size={12} className={refreshBusy ? 'spin-icon' : ''} />
          </button>
          <button
            className="glass-pill compact-action-btn"
            onClick={onWidget}
            title="Switch to widget view"
            style={{ width: 26, height: 26, minWidth: 26 }}
          >
            <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
              <path d="M9 1H3.5M9 1V6.5M9 1L1 9" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" strokeLinejoin="round" transform="rotate(180 5 5)"/>
            </svg>
          </button>
          <button type="button" aria-label="Minimize" title="Minimize" className="window-btn"
            onClick={() => { if (isTauriRuntime()) getCurrentWindow().minimize(); }}
            style={{ background: '#febc2e' }}
          />
          <button type="button" aria-label="Close" title="Close" className="window-btn"
            onClick={() => { if (isTauriRuntime()) invoke('quit_app'); }}
            style={{ background: '#ff5f57' }}
          />
        </div>
      </div>

      {/* Tab strip */}
      <div className="nav-tabs">
        <button
          className={`nav-tab ${activePage === 'overview' ? 'active' : ''}`}
          onClick={() => onNavigate('overview')}
          title="Overview"
        >
          <Home size={12} />
          Home
        </button>

        {providers.map(({ id, label, tint }) => {
          if (!enabledProviders[id]) return null;
          const health = statuses[id]?.health;
          const healthClass = health ? `health-${health}` : 'health-unknown';
          return (
            <button
              key={id}
              className={`nav-tab ${activePage === id ? `active tint-${tint}` : ''}`}
              onClick={() => onNavigate(id)}
              title={label}
            >
              <ProviderLogo provider={id} size={12} />
              {label}
              <span className={`nav-tab-dot ${healthClass}`} />
            </button>
          );
        })}

        <button
          className={`nav-tab ${activePage === 'settings' ? 'active' : ''}`}
          onClick={() => onNavigate('settings')}
          title="Settings (Ctrl+,)"
        >
          <Settings2 size={12} />
        </button>
      </div>
    </div>
  );
};

export default NavBar;
