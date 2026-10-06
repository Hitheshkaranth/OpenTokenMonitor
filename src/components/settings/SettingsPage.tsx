import { useState, useEffect, useMemo } from 'react';
import { Bell, Download, MenuSquare, Monitor, Palette, Power, RefreshCw, Server, Wallet } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import GlassToggle from '@/components/glass/GlassToggle';
import GlassInput from '@/components/glass/GlassInput';
import GlassButton from '@/components/glass/GlassButton';
import DiagnosticsPanel from '@/components/states/DiagnosticsPanel';
import ProviderLogo from '@/components/providers/ProviderLogo';
import AboutPanel from '@/components/settings/AboutPanel';
import { Budget, BudgetForecast, ExportFormat, PerProviderThresholds, ProviderId, ProviderStatus, RefreshCadence, TrayTitleMode } from '@/types';
import { useSettingsStore } from '@/stores/settingsStore';
import { useUsageStore } from '@/stores/usageStore';

const providers: ProviderId[] = ['claude', 'codex', 'antigravity'];

const providerLabels: Record<ProviderId, string> = {
  claude: 'Claude',
  codex: 'Codex',
  antigravity: 'Antigravity',
};

const placeholders: Record<ProviderId, string> = {
  claude: 'Auto-detected from ~/.claude',
  codex: 'Auto-detected from ~/.codex',
  antigravity: 'Antigravity API key (for OAuth)',
};

const providerAccents: Record<ProviderId, string> = {
  claude: '217 119 87',
  codex: '16 163 127',
  antigravity: '79 107 237',
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

const BudgetForecastSummary = ({ forecast }: { forecast: BudgetForecast }) => {
  const exceed = forecast.projected_exceeds_budget;
  return (
    <div className="stg-pp-forecast-wrap">
      <div className="stg-pp-forecast">
        <div className="stg-pp-forecast-item">
          <span className="stg-pp-forecast-item-label">Spend</span>
          <span className="stg-pp-forecast-item-value">${forecast.spend_to_date_usd.toFixed(2)}</span>
        </div>
        <div className="stg-pp-forecast-item">
          <span className="stg-pp-forecast-item-label">Proj</span>
          <span className="stg-pp-forecast-item-value">${forecast.projected_spend_usd.toFixed(2)}</span>
        </div>
        <div className={`stg-pp-forecast-item ${exceed ? 'stg-pp-forecast-exceed' : ''}`}>
          <span className="stg-pp-forecast-item-label">Used</span>
          <span className="stg-pp-forecast-item-value">{forecast.utilization_percent.toFixed(0)}%</span>
        </div>
        {exceed && <span className="stg-pp-forecast-flag">exceeds</span>}
      </div>
    </div>
  );
};

const SettingsPage = () => {
  const [view, setView] = useState<SettingsView>('settings');
  const [exportFormat, setExportFormat] = useState<ExportFormat>('csv');
  const [exportDays, setExportDays] = useState(30);

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

  const notificationsEnabled = useSettingsStore((s) => s.notificationsEnabled);
  const setNotificationsEnabled = useSettingsStore((s) => s.setNotificationsEnabled);
  const perProviderThresholds = useSettingsStore((s) => s.perProviderThresholds);
  const setProviderThresholds = useSettingsStore((s) => s.setProviderThresholds);
  const budgets = useSettingsStore((s) => s.budgets);
  const setBudgets = useSettingsStore((s) => s.setBudgets);
  const perProviderCadence = useSettingsStore((s) => s.perProviderCadence);
  const setPerProviderCadence = useSettingsStore((s) => s.setPerProviderCadence);
  const trayTitleMode = useSettingsStore((s) => s.trayTitleMode);
  const setTrayTitleMode = useSettingsStore((s) => s.setTrayTitleMode);

  const snapshots = useUsageStore((s) => s.snapshots);
  const costHistory = useUsageStore((s) => s.costHistory);
  const statuses = useUsageStore((s) => s.statuses);
  const alerts = useUsageStore((s) => s.alerts);
  const error = useUsageStore((s) => s.error);
  const backendBudgets = useUsageStore((s) => s.backendBudgets);
  const budgetsForecast = useUsageStore((s) => s.budgetsForecast);
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

  // Live previews of what each menu-bar mode would show, mirroring the
  // backend's format_tray_title (cost rows are keyed by UTC day).
  const trayPreviews = useMemo(() => {
    const letters: Record<ProviderId, string> = { claude: 'C', codex: 'X', antigravity: 'A' };
    const percent = providers
      .filter((p) => snapshots[p])
      .map((p) => `${letters[p]} ${Math.round(snapshots[p]?.windows[0]?.utilization ?? 0)}%`)
      .slice(0, 2)
      .join(' · ');
    const today = new Date().toISOString().slice(0, 10);
    const cost = providers
      .flatMap((p) => costHistory[p] ?? [])
      .filter((entry) => entry.date === today)
      .reduce((sum, entry) => sum + entry.estimated_cost_usd, 0);
    return { off: 'icon only', percent: percent || 'C 0%', cost: `$${cost.toFixed(2)} today` } satisfies Record<TrayTitleMode, string>;
  }, [snapshots, costHistory]);

  const trayTitleOptions: { value: TrayTitleMode; label: string }[] = [
    { value: 'off', label: 'Off' },
    { value: 'percent', label: 'Usage %' },
    { value: 'cost', label: "Today's cost" },
  ];

  const saveKey = async (provider: ProviderId) => {
    const key = apiKeys[provider];
    await setApiKeyRemote(provider, key);
    await refreshProvider(provider);
  };

  // A1 — optimistically update the store, then persist the per-provider alert
  // lines. The setter command name is owned by the backend agent; keep working
  // locally if it is unavailable.
  const applyProviderThreshold = async (provider: ProviderId, thresholds: PerProviderThresholds) => {
    setProviderThresholds(provider, thresholds);
    try {
      await invoke('set_thresholds', {
        provider,
        warning: thresholds.warning,
        high: thresholds.high,
        critical: thresholds.critical,
      });
    } catch (err) {
      console.warn('set_thresholds unavailable', err);
    }
  };

  // A3 — persist the per-provider spend cap.
  const applyProviderBudget = async (provider: ProviderId, budget: Budget) => {
    setBudgets(provider, budget);
    try {
      await invoke('set_budget', {
        provider,
        amount_usd: budget.amount_usd,
        period_days: budget.period_days,
      });
    } catch (err) {
      console.warn('set_budget unavailable', err);
    }
  };

  // A4 — persist the per-provider refresh cadence alongside the global one.
  const applyProviderCadence = async (provider: ProviderId, cadence: RefreshCadence) => {
    setPerProviderCadence(provider, cadence);
    try {
      await invoke('set_per_provider_cadence', { provider, cadence });
    } catch (err) {
      console.warn('set_provider_cadence unavailable', err);
    }
  };

  // Load persisted backend budgets once, then (re)forecast each provider
  // whenever the effective budget inputs change so the cards stay current.
  const budgetForecastKey = useMemo(
    () =>
      providers
        .map((provider) => `${provider}:${(backendBudgets[provider] ?? budgets[provider])?.period_days ?? ''}`)
        .join('|'),
    [backendBudgets, budgets]
  );

  useEffect(() => {
    void useUsageStore.getState().fetchBudgets();
  }, []);

  useEffect(() => {
    providers.forEach((provider) => {
      const effective = backendBudgets[provider] ?? budgets[provider];
      if (!effective || effective.amount_usd <= 0) return;
      void useUsageStore.getState().fetchBudgetForecast(provider, effective.period_days);
    });
  }, [budgetForecastKey]);

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

          {/* Menu bar title */}
          <div className="stg-section">
            <div className="stg-section-head">
              <div className="stg-section-head-left">
                <span className="stg-section-icon"><MenuSquare size={13} /></span>
                <span className="stg-section-title">Menu bar</span>
              </div>
              <span className="stg-badge">macOS</span>
            </div>
            <div className="stg-cadence-row">
              {trayTitleOptions.map((option) => {
                const isActive = trayTitleMode === option.value;
                return (
                  <button
                    key={option.value}
                    className={`stg-cadence-card ${isActive ? 'stg-cadence-card-active' : ''}`}
                    onClick={() => setTrayTitleMode(option.value)}
                    title="Shows live usage next to the menu-bar icon"
                  >
                    <span className="stg-tray-preview">{trayPreviews[option.value]}</span>
                    <span className="stg-cadence-label">{option.label}</span>
                  </button>
                );
              })}
            </div>
          </div>

          {/* Notifications + alert thresholds */}
          <div className="stg-section">
            <div className="stg-section-head">
              <div className="stg-section-head-left">
                <span className="stg-section-icon"><Bell size={13} /></span>
                <span className="stg-section-title">Notifications</span>
              </div>
              <span className="stg-badge">{notificationsEnabled ? 'On' : 'Off'}</span>
            </div>
            <div className="stg-pp-toggle-row">
              <span className="stg-pp-toggle-text">
                {notificationsEnabled
                  ? 'Once per threshold crossing, when on pace to hit a limit before reset, and on projected budget overrun'
                  : 'Notifications are disabled'}
              </span>
              <GlassToggle
                checked={notificationsEnabled}
                onChange={setNotificationsEnabled}
                label={notificationsEnabled ? 'On' : 'Off'}
              />
            </div>
            <div className="stg-note">
              <Bell size={11} />
              <span>
                Notifications require the OS notification permission. Grant access for OpenToken Monitor in
                System Settings → Privacy & Security → Notifications if alerts do not appear.
              </span>
            </div>
            <div className="stg-pp-grid">
              {providers.map((provider) => {
                const thresholds = perProviderThresholds[provider];
                return (
                  <div
                    key={provider}
                    className="stg-pp-card"
                    style={{ '--widget-accent': providerAccents[provider] } as React.CSSProperties}
                  >
                    <div className="stg-pp-identity">
                      <ProviderLogo provider={provider} size={14} />
                      <span className="stg-pp-name">{providerLabels[provider]}</span>
                    </div>
                    <div className="stg-pp-fields stg-pp-fields-threshold">
                      {(['warning', 'high', 'critical'] as const).map((field) => (
                        <div key={field} className="stg-pp-field">
                          <span className="stg-pp-field-label">{field}</span>
                          <GlassInput
                            type="number"
                            min={0}
                            max={100}
                            value={String(thresholds[field])}
                            onChange={(next) =>
                              applyProviderThreshold(provider, {
                                ...thresholds,
                                [field]: Math.min(100, Math.max(0, Number(next) || thresholds[field])),
                              })
                            }
                            style={{ minWidth: 60 }}
                          />
                        </div>
                      ))}
                    </div>
                  </div>
                );
              })}
            </div>
          </div>

          {/* Budgets */}
          <div className="stg-section">
            <div className="stg-section-head">
              <div className="stg-section-head-left">
                <span className="stg-section-icon"><Wallet size={13} /></span>
                <span className="stg-section-title">Budgets</span>
              </div>
              <span className="stg-badge">spend cap</span>
            </div>
            <div className="stg-pp-grid">
              {providers.map((provider) => {
                const budget = budgets[provider];
                return (
                  <div
                    key={provider}
                    className="stg-pp-card"
                    style={{ '--widget-accent': providerAccents[provider] } as React.CSSProperties}
                  >
                    <div className="stg-pp-identity">
                      <ProviderLogo provider={provider} size={14} />
                      <span className="stg-pp-name">{providerLabels[provider]}</span>
                    </div>
                    <div className="stg-pp-fields stg-pp-fields-budget">
                      <div className="stg-pp-field">
                        <span className="stg-pp-field-label">Amount ($)</span>
                        <GlassInput
                          type="number"
                          min={0}
                          value={String(budget.amount_usd)}
                          onChange={(next) =>
                            applyProviderBudget(provider, { ...budget, amount_usd: Math.max(0, Number(next) || 0) })
                          }
                          style={{ minWidth: 66 }}
                        />
                      </div>
                      <div className="stg-pp-field">
                        <span className="stg-pp-field-label">Period (days)</span>
                        <GlassInput
                          type="number"
                          min={1}
                          value={String(budget.period_days)}
                          onChange={(next) =>
                            applyProviderBudget(provider, { ...budget, period_days: Math.max(1, Number(next) || 1) })
                          }
                          style={{ minWidth: 66 }}
                        />
                      </div>
                    </div>

                    {budgetsForecast[provider] && (
                      <div className="stg-pp-forecast-wrap">
                        <BudgetForecastSummary forecast={budgetsForecast[provider]} />
                      </div>
                    )}
                  </div>
                );
              })}
            </div>
          </div>

          {/* Per-provider refresh cadence */}
          <div className="stg-section">
            <div className="stg-section-head">
              <div className="stg-section-head-left">
                <span className="stg-section-icon"><RefreshCw size={13} /></span>
                <span className="stg-section-title">Provider cadence</span>
              </div>
              <span className="stg-badge">per source</span>
            </div>
            <div className="stg-pp-grid">
              {providers.map((provider) => {
                const cadence = perProviderCadence[provider];
                return (
                  <div
                    key={provider}
                    className="stg-pp-card"
                    style={{ '--widget-accent': providerAccents[provider] } as React.CSSProperties}
                  >
                    <div className="stg-pp-identity">
                      <ProviderLogo provider={provider} size={14} />
                      <span className="stg-pp-name">{providerLabels[provider]}</span>
                    </div>
                    <div className="stg-pp-fields stg-pp-field-select">
                      <div className="stg-pp-field">
                        <span className="stg-pp-field-label">Refresh</span>
                        <select
                          className="stg-cadence-select"
                          value={cadence}
                          onChange={(event) => applyProviderCadence(provider, event.target.value as RefreshCadence)}
                        >
                          {cadenceOptions.map((option) => (
                            <option key={option.value} value={option.value}>
                              {option.label}
                            </option>
                          ))}
                        </select>
                      </div>
                    </div>
                  </div>
                );
              })}
            </div>
          </div>

          {/* Export report */}
          <div className="stg-section">
            <div className="stg-section-head">
              <div className="stg-section-head-left">
                <span className="stg-section-icon"><Download size={13} /></span>
                <span className="stg-section-title">Export</span>
              </div>
              <span className="stg-badge">CSV · JSON · PDF</span>
            </div>
            <div className="stg-pp-fields stg-pp-fields-budget">
              <div className="stg-pp-field">
                <span className="stg-pp-field-label">Format</span>
                <select
                  className="stg-cadence-select"
                  value={exportFormat}
                  onChange={(event) => setExportFormat(event.target.value as ExportFormat)}
                >
                  <option value="csv">CSV</option>
                  <option value="json">JSON</option>
                  <option value="pdf">PDF</option>
                </select>
              </div>
              <div className="stg-pp-field">
                <span className="stg-pp-field-label">Days</span>
                <select
                  className="stg-cadence-select"
                  value={String(exportDays)}
                  onChange={(event) => setExportDays(Number(event.target.value))}
                >
                  <option value="7">7</option>
                  <option value="30">30</option>
                  <option value="90">90</option>
                </select>
              </div>
              <div className="stg-pp-field stg-pp-field-btn">
                <button
                  type="button"
                  className="stg-pp-export-btn"
                  onClick={() => void useUsageStore.getState().exportReport(exportFormat, exportDays)}
                >
                  Export
                </button>
              </div>
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
