import { useState } from 'react';
import { Monitor, Palette, Power, RefreshCw, Server } from 'lucide-react';
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

const providerAccents: Record<ProviderId, string> = {
  claude: '217 119 87',
  codex: '16 163 127',
  gemini: '66 133 244',
};

const themeOptions = [
  { value: 'system', label: 'System', note: 'Follow OS appearance', tag: 'Adaptive' },
  { value: 'dark', label: 'Dark', note: 'Glass-heavy contrast', tag: 'Low glare' },
  { value: 'light', label: 'Light', note: 'Bright desktop mode', tag: 'Airy' },
] as const;

const cadenceOptions: { value: RefreshCadence; label: string; note: string; badge: string; bars: number[] }[] = [
  { value: 'manual', label: 'Manual', note: 'On demand', badge: 'Hold', bars: [8, 14, 10, 6] },
  { value: 'every30s', label: '30s', note: 'Fastest', badge: 'Rapid', bars: [28, 38, 30, 20] },
  { value: 'every1m', label: '1m', note: 'Balanced', badge: 'Live', bars: [24, 34, 28, 18] },
  { value: 'every2m', label: '2m', note: 'Moderate', badge: 'Balanced', bars: [18, 28, 22, 14] },
  { value: 'every5m', label: '5m', note: 'Low-touch', badge: 'Light', bars: [12, 18, 14, 10] },
  { value: 'every15m', label: '15m', note: 'Minimal', badge: 'Quiet', bars: [8, 12, 9, 6] },
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

  const surfaceCards = [
    { key: 'theme', label: 'Theme', value: formatThemeLabel(theme), badge: theme === 'system' ? 'OS' : 'Fixed', icon: Palette },
    { key: 'mode', label: 'Mode', value: widgetMode ? 'Widget' : 'Dashboard', badge: widgetMode ? 'Compact' : 'Full', icon: Monitor },
    { key: 'refresh', label: 'Refresh', value: formatCadenceLabel(refreshCadence), badge: refreshCadence === 'manual' ? 'Manual' : 'Auto', icon: RefreshCw },
    { key: 'providers', label: 'Sources', value: `${enabledCount}/3`, badge: `${activeCount} live`, icon: Server },
  ] as const;

  const saveKey = async (provider: ProviderId) => {
    const key = apiKeys[provider];
    await setApiKeyRemote(provider, key);
    await refreshProvider(provider);
  };

  return (
    <div className="stg-page">
      {/* Settings / About toggle */}
      <div className="stg-toggle-row">
        <button
          className={`stg-toggle-btn ${view === 'settings' ? 'stg-toggle-active' : ''}`}
          onClick={() => setView('settings')}
        >
          Settings
        </button>
        <button
          className={`stg-toggle-btn ${view === 'about' ? 'stg-toggle-active' : ''}`}
          onClick={() => setView('about')}
        >
          About
        </button>
      </div>

      {view === 'settings' ? (
        <>
          {/* Control surface metrics */}
          <div className="stg-metrics-row">
            {surfaceCards.map((card) => {
              const Icon = card.icon;
              return (
                <div key={card.key} className="stg-metric-card">
                  <span className="stg-metric-icon">
                    <Icon size={13} strokeWidth={2.2} />
                  </span>
                  <div className="stg-metric-copy">
                    <span className="stg-metric-label">{card.label}</span>
                    <span className="stg-metric-value">{card.value}</span>
                  </div>
                  <span className="stg-metric-badge">{card.badge}</span>
                </div>
              );
            })}
          </div>

          {/* Appearance */}
          <div className="stg-section">
            <div className="stg-section-head">
              <div className="stg-section-head-left">
                <span className="stg-section-icon"><Palette size={13} /></span>
                <span className="stg-section-title">Appearance</span>
              </div>
              <span className="stg-badge">{formatThemeLabel(theme)}</span>
            </div>
            <div className="stg-theme-row">
              {themeOptions.map((option) => {
                const isActive = theme === option.value;
                return (
                  <button
                    key={option.value}
                    className={`stg-theme-card ${isActive ? 'stg-theme-card-active' : ''}`}
                    onClick={() => setTheme(option.value)}
                    title={option.note}
                  >
                    <div className={`stg-theme-preview stg-theme-preview-${option.value}`}>
                      <span className="stg-theme-preview-bar" />
                      <div className="stg-theme-preview-cols">
                        <span className="stg-theme-preview-panel" />
                        <span className="stg-theme-preview-panel stg-theme-preview-side" />
                      </div>
                    </div>
                    <span className="stg-theme-label">{option.label}</span>
                    <span className="stg-theme-tag">{isActive ? 'Active' : option.tag}</span>
                  </button>
                );
              })}
            </div>
          </div>

          {/* Refresh Cadence */}
          <div className="stg-section">
            <div className="stg-section-head">
              <div className="stg-section-head-left">
                <span className="stg-section-icon"><RefreshCw size={13} /></span>
                <span className="stg-section-title">Refresh</span>
              </div>
              <span className="stg-badge">{formatCadenceLabel(refreshCadence)}</span>
            </div>
            <div className="stg-cadence-row">
              {cadenceOptions.map((option) => {
                const isActive = refreshCadence === option.value;
                return (
                  <button
                    key={option.value}
                    className={`stg-cadence-card ${isActive ? 'stg-cadence-card-active' : ''}`}
                    onClick={() => { setRefreshCadence(option.value); setCadenceRemote(option.value); }}
                    title={option.note}
                  >
                    <div className="stg-cadence-bars">
                      {option.bars.map((h, i) => (
                        <span key={i} className="stg-cadence-bar" style={{ height: h }} />
                      ))}
                    </div>
                    <span className="stg-cadence-label">{option.label}</span>
                  </button>
                );
              })}
            </div>
          </div>

          {/* Desktop Behavior */}
          <div className="stg-section">
            <div className="stg-section-head">
              <div className="stg-section-head-left">
                <span className="stg-section-icon"><Power size={13} /></span>
                <span className="stg-section-title">Startup</span>
              </div>
              <span className="stg-badge">{launchAtStartup ? 'Auto' : 'Manual'}</span>
            </div>
            <div className="stg-startup-row">
              <span className="stg-startup-text">
                {launchAtStartup ? 'Starts automatically after sign-in' : 'Manual launch only'}
              </span>
              <GlassToggle
                checked={launchAtStartup}
                onChange={setLaunchAtStartup}
                label={launchAtStartup ? 'On' : 'Off'}
              />
            </div>
          </div>

          {/* Providers */}
          <div className="stg-section">
            <div className="stg-section-head">
              <div className="stg-section-head-left">
                <span className="stg-section-icon"><Server size={13} /></span>
                <span className="stg-section-title">Providers</span>
              </div>
              <span className="stg-badge">{enabledCount} enabled</span>
            </div>
            <div className="stg-providers">
              {providers.map((provider) => {
                const snapshot = snapshots[provider];
                const status = statuses[provider];
                const providerAlertCount = alerts[provider].length;
                const isEnabled = enabledProviders[provider];

                return (
                  <div
                    key={provider}
                    className="stg-provider-card"
                    style={{ '--widget-accent': providerAccents[provider] } as React.CSSProperties}
                  >
                    <div className="stg-provider-top">
                      <ProviderLogo provider={provider} size={16} />
                      <span className="stg-provider-name">{providerLabels[provider]}</span>
                      <span className="stg-badge" style={{ color: status?.health === 'active' ? '#34d399' : status?.health === 'error' ? '#f87171' : '#fbbf24' }}>
                        {healthLabel(status?.health)}
                      </span>
                      <span className="stg-badge">{providerAlertCount} alerts</span>
                      <GlassToggle
                        checked={isEnabled}
                        onChange={(next) => setProviderEnabled(provider, next)}
                        label={isEnabled ? 'On' : 'Off'}
                      />
                    </div>
                    <div className="stg-provider-key-row">
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
                    <div className="stg-provider-footer">
                      <span>{formatFetchedAt(snapshot?.fetched_at)}</span>
                      <span>{snapshot?.stale ? 'stale' : 'live'}</span>
                      {snapshot && <span>{snapshot.source}</span>}
                    </div>
                  </div>
                );
              })}
            </div>
          </div>

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
