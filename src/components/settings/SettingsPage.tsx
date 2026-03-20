import { useState } from 'react';
import { Monitor, Palette, Power, RefreshCw, Server } from 'lucide-react';
import GlassPanel from '@/components/glass/GlassPanel';
import GlassToggle from '@/components/glass/GlassToggle';
import GlassInput from '@/components/glass/GlassInput';
import GlassButton from '@/components/glass/GlassButton';
import DiagnosticsPanel from '@/components/states/DiagnosticsPanel';
import ProviderLogo from '@/components/providers/ProviderLogo';
import AboutPanel from '@/components/settings/AboutPanel';
import { ProviderId, ProviderStatus, RefreshCadence } from '@/types';
import { useSettingsStore } from '@/stores/settingsStore';
import { useUsageStore } from '@/stores/usageStore';

const providers: ProviderId[] = ['claude', 'codex', 'gemini'];

const providerLabels: Record<ProviderId, string> = {
  claude: 'Claude',
  codex: 'Codex',
  gemini: 'Gemini',
};

const placeholders: Record<ProviderId, string> = {
  claude: 'Auto-detected from ~/.claude',
  codex: 'Auto-detected from ~/.codex',
  gemini: 'Gemini API key',
};

const providerTint: Record<ProviderId, 'claude' | 'codex' | 'gemini'> = {
  claude: 'claude',
  codex: 'codex',
  gemini: 'gemini',
};

const themeOptions = [
  { value: 'system', label: 'System', note: 'Follow OS appearance', tag: 'Adaptive' },
  { value: 'dark', label: 'Dark', note: 'Glass-heavy contrast', tag: 'Low glare' },
  { value: 'light', label: 'Light', note: 'Bright desktop mode', tag: 'Airy' },
] as const;

const cadenceOptions: { value: RefreshCadence; label: string; note: string; badge: string; bars: number[] }[] = [
  { value: 'manual', label: 'Manual', note: 'Refresh only on demand', badge: 'Hold', bars: [8, 14, 10, 6] },
  { value: 'every30s', label: '30s', note: 'Fastest live polling', badge: 'Rapid', bars: [28, 38, 30, 20] },
  { value: 'every1m', label: '1m', note: 'Balanced default cadence', badge: 'Live', bars: [24, 34, 28, 18] },
  { value: 'every2m', label: '2m', note: 'Reduced background work', badge: 'Balanced', bars: [18, 28, 22, 14] },
  { value: 'every5m', label: '5m', note: 'Low-touch refresh', badge: 'Light', bars: [12, 18, 14, 10] },
  { value: 'every15m', label: '15m', note: 'Minimal polling', badge: 'Quiet', bars: [8, 12, 9, 6] },
];

type SettingsView = 'settings' | 'about';

const healthLabel = (health?: ProviderStatus['health']) => {
  if (health === 'active') return 'active';
  if (health === 'error') return 'error';
  return 'waiting';
};

const formatCadenceLabel = (cadence: RefreshCadence) =>
  cadenceOptions.find((option) => option.value === cadence)?.label ?? cadence;

const formatThemeLabel = (theme: 'system' | 'dark' | 'light') =>
  themeOptions.find((option) => option.value === theme)?.label ?? theme;

const formatFetchedAt = (value?: string) => {
  if (!value) return 'No snapshot yet';
  return `Updated ${new Date(value).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}`;
};

const SettingsPage = () => {
  const [view, setView] = useState<SettingsView>('settings');

  const theme = useSettingsStore((s) => s.theme);
  const setTheme = useSettingsStore((s) => s.setTheme);
  const widgetMode = useSettingsStore((s) => s.widgetMode);
  const enabledProviders = useSettingsStore((s) => s.enabledProviders);
  const setProviderEnabled = useSettingsStore((s) => s.setProviderEnabled);
  const apiKeys = useSettingsStore((s) => s.apiKeys);
  const setApiKeyLocal = useSettingsStore((s) => s.setApiKey);
  const refreshCadence = useSettingsStore((s) => s.refreshCadence);
  const setRefreshCadence = useSettingsStore((s) => s.setRefreshCadence);
  const launchAtStartup = useSettingsStore((s) => s.launchAtStartup);
  const setLaunchAtStartup = useSettingsStore((s) => s.setLaunchAtStartup);

  const snapshots = useUsageStore((s) => s.snapshots);
  const statuses = useUsageStore((s) => s.statuses);
  const alerts = useUsageStore((s) => s.alerts);
  const error = useUsageStore((s) => s.error);
  const setApiKeyRemote = useUsageStore((s) => s.setApiKey);
  const setCadenceRemote = useUsageStore((s) => s.setCadence);
  const refreshProvider = useUsageStore((s) => s.refreshProvider);

  const enabledCount = providers.filter((provider) => enabledProviders[provider]).length;
  const activeCount = providers.filter((provider) => statuses[provider]?.health === 'active').length;
  const currentCadence = cadenceOptions.find((option) => option.value === refreshCadence);

  const surfaceCards = [
    {
      key: 'theme',
      label: 'Theme',
      value: formatThemeLabel(theme),
      badge: theme === 'system' ? 'OS' : 'Fixed',
      icon: Palette,
      className: 'settings-metric-card-theme',
    },
    {
      key: 'mode',
      label: 'Mode',
      value: widgetMode ? 'Widget' : 'Dashboard',
      badge: widgetMode ? 'Compact' : 'Full',
      icon: Monitor,
      className: 'settings-metric-card-mode',
    },
    {
      key: 'refresh',
      label: 'Refresh',
      value: formatCadenceLabel(refreshCadence),
      badge: refreshCadence === 'manual' ? 'Manual' : 'Auto',
      icon: RefreshCw,
      className: 'settings-metric-card-refresh',
    },
    {
      key: 'providers',
      label: 'Sources',
      value: `${enabledCount}/3 enabled`,
      badge: `${activeCount} live`,
      icon: Server,
      className: 'settings-metric-card-providers',
    },
  ] as const;

  const saveKey = async (provider: ProviderId) => {
    const key = apiKeys[provider];
    await setApiKeyRemote(provider, key);
    await refreshProvider(provider);
  };

  return (
    <div className="settings-page">
      <div className="settings-topbar">
        <div className="segment-bar settings-segment">
          <button
            type="button"
            className={`segment-btn ${view === 'settings' ? 'segment-btn-active' : ''}`}
            onClick={() => setView('settings')}
          >
            Settings
          </button>
          <button
            type="button"
            className={`segment-btn ${view === 'about' ? 'segment-btn-active' : ''}`}
            onClick={() => setView('about')}
          >
            About
          </button>
        </div>
      </div>

      {view === 'settings' ? (
        <>
          <GlassPanel className="settings-hero">
            <div className="settings-surface-header">
              <div className="settings-section-title">Control Surface</div>
              <span className="glass-pill settings-inline-badge">Live profile</span>
            </div>
            <div className="settings-hero-metrics">
              {surfaceCards.map((card) => {
                const Icon = card.icon;

                return (
                  <div key={card.key} className={`settings-metric-card ${card.className}`}>
                    <span className="settings-metric-icon">
                      <Icon size={14} strokeWidth={2.2} />
                    </span>
                    <div className="settings-metric-copy">
                      <span className="settings-metric-label">{card.label}</span>
                      <span className="settings-metric-value">{card.value}</span>
                    </div>
                    <span className="glass-pill settings-metric-badge">{card.badge}</span>
                  </div>
                );
              })}
            </div>
          </GlassPanel>

          <GlassPanel className="settings-section settings-controls-panel">
            <div className="settings-block-header settings-controls-header">
              <div className="settings-section-title">Appearance &amp; Refresh</div>
              <div className="settings-control-live-row">
                <span className="glass-pill settings-inline-badge settings-control-live-pill">{formatThemeLabel(theme)}</span>
                <span className="glass-pill settings-inline-badge settings-control-live-pill">{formatCadenceLabel(refreshCadence)}</span>
              </div>
            </div>
            <div className="settings-designer-grid">
              <div className="settings-design-board settings-theme-board">
                <div className="settings-design-header">
                  <span className="settings-control-icon settings-design-icon">
                    <Palette size={15} strokeWidth={2.2} />
                  </span>
                  <div className="settings-design-copy">
                    <span className="settings-control-title">Appearance</span>
                    <span className="settings-design-current">{formatThemeLabel(theme)} profile</span>
                  </div>
                </div>
                <div className="settings-theme-grid">
                  {themeOptions.map((option) => {
                    const isActive = theme === option.value;

                    return (
                      <button
                        key={option.value}
                        type="button"
                        className={`settings-theme-card settings-theme-card-${option.value} ${isActive ? 'settings-theme-card-active' : ''}`.trim()}
                        onClick={() => setTheme(option.value)}
                        title={option.note}
                      >
                        <div className="settings-theme-card-head">
                          <span className="settings-theme-card-title">{option.label}</span>
                          <span className="glass-pill settings-theme-card-chip">
                            {isActive ? 'Active' : option.tag}
                          </span>
                        </div>
                        <div className={`settings-theme-preview settings-theme-preview-${option.value}`} aria-hidden="true">
                          <span className="settings-theme-preview-toolbar" />
                          <div className="settings-theme-preview-columns">
                            <span className="settings-theme-preview-panel settings-theme-preview-panel-main" />
                            <span className="settings-theme-preview-panel settings-theme-preview-panel-side" />
                          </div>
                          <div className="settings-theme-preview-footer">
                            <span />
                            <span />
                          </div>
                        </div>
                        <span className="settings-theme-note">{option.note}</span>
                      </button>
                    );
                  })}
                </div>
              </div>

              <div className="settings-design-board settings-refresh-board">
                <div className="settings-design-header">
                  <span className="settings-control-icon settings-design-icon">
                    <RefreshCw size={15} strokeWidth={2.2} />
                  </span>
                  <div className="settings-design-copy">
                    <span className="settings-control-title">Refresh</span>
                    <span className="settings-design-current">{currentCadence?.note ?? 'Polling profile'}</span>
                  </div>
                </div>
                <div className="settings-refresh-grid">
                  {cadenceOptions.map((option) => {
                    const isActive = refreshCadence === option.value;

                    return (
                      <button
                        key={option.value}
                        type="button"
                        className={`settings-refresh-card settings-refresh-card-${option.value} ${isActive ? 'settings-refresh-card-active' : ''}`.trim()}
                        onClick={() => {
                          setRefreshCadence(option.value);
                          setCadenceRemote(option.value);
                        }}
                        title={option.note}
                      >
                        <div className="settings-refresh-card-head">
                          <span className="settings-refresh-title">{option.label}</span>
                          <span className="glass-pill settings-refresh-chip">
                            {isActive ? 'Selected' : option.badge}
                          </span>
                        </div>
                        <div className="settings-refresh-visual" aria-hidden="true">
                          {option.bars.map((height, index) => (
                            <span
                              key={`${option.value}-${index}`}
                              className="settings-refresh-bar"
                              style={{ height }}
                            />
                          ))}
                        </div>
                        <span className="settings-refresh-note">{option.note}</span>
                      </button>
                    );
                  })}
                </div>
              </div>
            </div>
          </GlassPanel>

          <GlassPanel className="settings-section settings-startup-shell">
            <div className="settings-block-header">
              <div className="settings-section-title">Desktop Behavior</div>
              <span className="glass-pill settings-inline-badge">
                {launchAtStartup ? 'Auto-start on' : 'Auto-start off'}
              </span>
            </div>
            <div className="settings-startup-card">
              <span className="settings-control-icon settings-startup-icon">
                <Power size={15} strokeWidth={2.2} />
              </span>
              <div className="settings-startup-copy">
                <span className="settings-control-title">Launch at startup</span>
                <span className="settings-startup-note">
                  {launchAtStartup
                    ? 'OpenTokenMonitor will start automatically after sign-in and stay in the tray on auto-launch.'
                    : 'OpenTokenMonitor will only run when you start it manually.'}
                </span>
              </div>
              <GlassToggle
                checked={launchAtStartup}
                onChange={setLaunchAtStartup}
                label={launchAtStartup ? 'On' : 'Off'}
              />
            </div>
          </GlassPanel>

          <GlassPanel className="settings-section settings-provider-shell">
            <div className="settings-block-header">
              <div className="settings-section-title">Providers</div>
              <span className="glass-pill settings-inline-badge">{enabledCount} enabled</span>
            </div>
            <div className="settings-provider-list">
              {providers.map((provider) => {
                const snapshot = snapshots[provider];
                const status = statuses[provider];
                const providerAlertCount = alerts[provider].length;
                const isEnabled = enabledProviders[provider];

                return (
                  <GlassPanel key={provider} tint={providerTint[provider]} className="settings-provider-card">
                    <div className="settings-provider-card-top">
                      <div className="settings-provider-identity">
                        <ProviderLogo provider={provider} size={18} />
                        <div className="settings-provider-copy">
                          <div className="settings-provider-name">{providerLabels[provider]}</div>
                          <div className="settings-provider-description">{status?.message ?? placeholders[provider]}</div>
                        </div>
                      </div>
                      <GlassToggle
                        checked={isEnabled}
                        onChange={(next) => setProviderEnabled(provider, next)}
                        label={isEnabled ? 'On' : 'Off'}
                      />
                    </div>

                    <div className="settings-provider-tags">
                      <span className="glass-pill settings-tag">{healthLabel(status?.health)}</span>
                      {snapshot && <span className="glass-pill settings-tag">{snapshot.source}</span>}
                      <span className="glass-pill settings-tag">{providerAlertCount} alerts</span>
                    </div>

                    <div className="settings-key-row settings-provider-input-row">
                      <GlassInput
                        type="password"
                        value={apiKeys[provider]}
                        onChange={(value) => setApiKeyLocal(provider, value)}
                        placeholder={placeholders[provider]}
                        monospace
                      />
                      <GlassButton variant="primary" size="sm" onClick={() => saveKey(provider)}>
                        Save
                      </GlassButton>
                    </div>

                    <div className="settings-provider-footer">
                      <span className="metric-label">{formatFetchedAt(snapshot?.fetched_at)}</span>
                      <span className="metric-label">{snapshot?.stale ? 'stale snapshot' : 'live state'}</span>
                    </div>
                  </GlassPanel>
                );
              })}
            </div>
          </GlassPanel>

          <DiagnosticsPanel
            statuses={statuses}
            snapshots={snapshots}
            alerts={alerts}
            globalError={error}
          />
        </>
      ) : (
        <AboutPanel />
      )}
    </div>
  );
};

export default SettingsPage;
