import GlassPanel from '@/components/glass/GlassPanel';
import GlassToggle from '@/components/glass/GlassToggle';
import GlassInput from '@/components/glass/GlassInput';
import GlassButton from '@/components/glass/GlassButton';
import DiagnosticsPanel from '@/components/states/DiagnosticsPanel';
import ProviderLogo from '@/components/providers/ProviderLogo';
import { ProviderId, RefreshCadence } from '@/types';
import { useSettingsStore } from '@/stores/settingsStore';
import { useUsageStore } from '@/stores/usageStore';

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

const SettingsPage = () => {
  const theme = useSettingsStore((s) => s.theme);
  const setTheme = useSettingsStore((s) => s.setTheme);
  const widgetMode = useSettingsStore((s) => s.widgetMode);
  const setWidgetMode = useSettingsStore((s) => s.setWidgetMode);
  const demoMode = useSettingsStore((s) => s.demoMode);
  const setDemoMode = useSettingsStore((s) => s.setDemoMode);
  const enabledProviders = useSettingsStore((s) => s.enabledProviders);
  const setProviderEnabled = useSettingsStore((s) => s.setProviderEnabled);
  const apiKeys = useSettingsStore((s) => s.apiKeys);
  const setApiKeyLocal = useSettingsStore((s) => s.setApiKey);
  const refreshCadence = useSettingsStore((s) => s.refreshCadence);
  const setRefreshCadence = useSettingsStore((s) => s.setRefreshCadence);

  const snapshots = useUsageStore((s) => s.snapshots);
  const statuses = useUsageStore((s) => s.statuses);
  const alerts = useUsageStore((s) => s.alerts);
  const error = useUsageStore((s) => s.error);
  const setApiKeyRemote = useUsageStore((s) => s.setApiKey);
  const setCadenceRemote = useUsageStore((s) => s.setCadence);
  const refreshProvider = useUsageStore((s) => s.refreshProvider);

  const saveKey = async (provider: ProviderId) => {
    const key = apiKeys[provider];
    await setApiKeyRemote(provider, key);
    await refreshProvider(provider);
  };

  return (
    <div className="settings-page">
      <div className="settings-page-title">Settings</div>

      <GlassPanel className="settings-section">
        <div className="settings-section-title">Appearance</div>
        <div className="settings-row">
          <select
            className="glass-pill"
            value={theme}
            onChange={(e) => setTheme(e.target.value as 'light' | 'dark' | 'system')}
            style={{ fontSize: 10, padding: '2px 6px' }}
          >
            <option value="system">System</option>
            <option value="dark">Dark</option>
            <option value="light">Light</option>
          </select>
          <GlassToggle checked={widgetMode} onChange={setWidgetMode} label="Widget" />
          <GlassToggle checked={demoMode} onChange={setDemoMode} label="Demo" />
        </div>
      </GlassPanel>

      <GlassPanel className="settings-section">
        <div className="settings-section-title">Providers</div>
        {(['claude', 'codex', 'gemini'] as ProviderId[]).map((provider) => (
          <div key={provider} className="settings-provider-block">
            <div className="settings-provider-header">
              <ProviderLogo provider={provider} size={13} />
              <span style={{ fontWeight: 600, fontSize: 11 }}>{providerLabels[provider]}</span>
              <GlassToggle
                checked={enabledProviders[provider]}
                onChange={(next) => setProviderEnabled(provider, next)}
                label="On"
              />
            </div>
            <div className="settings-key-row">
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
          </div>
        ))}
      </GlassPanel>

      <GlassPanel className="settings-section">
        <div className="settings-section-title">Refresh</div>
        <div className="settings-row">
          <select
            className="glass-pill"
            value={refreshCadence}
            onChange={(e) => {
              const value = e.target.value as RefreshCadence;
              setRefreshCadence(value);
              setCadenceRemote(value);
            }}
            style={{ fontSize: 10, padding: '2px 6px' }}
          >
            <option value="manual">Manual</option>
            <option value="every30s">Every 30s</option>
            <option value="every1m">Every 1m</option>
            <option value="every2m">Every 2m</option>
            <option value="every5m">Every 5m</option>
            <option value="every15m">Every 15m</option>
          </select>
        </div>
      </GlassPanel>

      <DiagnosticsPanel
        statuses={statuses}
        snapshots={snapshots}
        alerts={alerts}
        globalError={error}
        demoMode={demoMode}
      />
    </div>
  );
};

export default SettingsPage;
