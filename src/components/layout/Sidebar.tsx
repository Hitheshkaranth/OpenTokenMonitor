import { FolderKanban, Home, RefreshCw, Settings2 } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import ProviderLogo from '@/components/providers/ProviderLogo';
import { PageId, ProviderId } from '@/types';
import { useSettingsStore } from '@/stores/settingsStore';
import { useUsageStore } from '@/stores/usageStore';
import { getProviderAccessState, providerAccessDotClass } from '@/utils/providerAccess';
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
  const snapshots = useUsageStore((s) => s.snapshots);

  return (
    <div className="nav-bar" data-tauri-drag-region>
      {/* Row 1: brand + actions + window controls */}
      <div className="nav-header" data-tauri-drag-region>
        <div className="nav-header-left" data-tauri-drag-region>
          <img src="/open_token_monitor_icon.png" alt="OTM" className="nav-logo" />
          <span className="nav-title" data-tauri-drag-region>OpenToken Monitor</span>
        </div>

        <div className="nav-header-right">
          <button
            className="nav-action-btn"
            onClick={onRefresh}
            disabled={refreshBusy}
            title="Refresh all (Ctrl+R)"
          >
            <RefreshCw size={11} className={refreshBusy ? 'spin-icon' : ''} />
          </button>
          <button
            className="nav-action-btn"
            onClick={onWidget}
            title="Switch to widget view"
          >
            <svg width="10" height="10" viewBox="0 0 12 12" fill="none">
              <path d="M9 1H3.5M9 1V6.5M9 1L1 9" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" strokeLinejoin="round" transform="rotate(180 5 5)"/>
            </svg>
          </button>
          <div className="nav-traffic-lights">
            <button type="button" aria-label="Minimize" title="Minimize" className="window-btn window-btn-minimize"
              onClick={() => { if (isTauriRuntime()) getCurrentWindow().minimize(); }}
            />
            <button type="button" aria-label="Close" title="Close" className="window-btn window-btn-close"
              onClick={() => { if (isTauriRuntime()) invoke('quit_app'); }}
            />
          </div>
        </div>
      </div>

      {/* Row 2: navigation — left pages | center providers | right settings */}
      <nav className="nav-pill-row" data-tauri-drag-region>
        <div className="nav-pill-group">
          <button
            className={`nav-pill ${activePage === 'overview' ? 'nav-pill-active' : ''}`}
            onClick={() => onNavigate('overview')}
            title="Home"
          >
            <Home size={13} />
          </button>
          <button
            className={`nav-pill ${activePage === 'projects' ? 'nav-pill-active' : ''}`}
            onClick={() => onNavigate('projects')}
            title="Projects"
          >
            <FolderKanban size={13} />
          </button>
        </div>

        <div className="nav-pill-group">
          {providers.map(({ id, label, tint }) => {
            if (!enabledProviders[id]) return null;
            const access = getProviderAccessState(statuses[id], snapshots[id]);
            const healthClass = providerAccessDotClass(access.health);
            return (
              <button
                key={id}
                className={`nav-pill ${activePage === id ? `nav-pill-active nav-pill-tint-${tint}` : ''}`}
                onClick={() => onNavigate(id)}
                title={`${label}: ${access.detail}`}
              >
                <ProviderLogo provider={id} size={14} />
                <span className={`nav-pill-dot ${healthClass}`} />
              </button>
            );
          })}
        </div>

        <div className="nav-pill-group">
          <button
            className={`nav-pill ${activePage === 'settings' ? 'nav-pill-active' : ''}`}
            onClick={() => onNavigate('settings')}
            title="Settings (Ctrl+,)"
          >
            <Settings2 size={13} />
          </button>
        </div>
      </nav>
    </div>
  );
};

export default NavBar;
