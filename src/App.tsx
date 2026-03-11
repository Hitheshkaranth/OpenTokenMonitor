import { useEffect, useMemo, useState } from 'react';
import { RefreshCw, TrendingUp } from 'lucide-react';
import AppShell from '@/components/layout/AppShell';
import WidgetMode from '@/components/layout/WidgetMode';
import ProviderCard from '@/components/providers/ProviderCard';
import ProviderOverview from '@/components/providers/ProviderOverview';
import GlassPanel from '@/components/glass/GlassPanel';
import GlassToggle from '@/components/glass/GlassToggle';
import { ProviderId, ProviderTab } from '@/types';
import { useSettingsStore } from '@/stores/settingsStore';
import { useUsageStore } from '@/stores/usageStore';
import { useUsageData } from '@/hooks/useUsageData';
import { useProviderStatus } from '@/hooks/useProviderStatus';
import { useGlassTheme } from '@/hooks/useGlassTheme';
import EmptyState from '@/components/states/EmptyState';
import ErrorBoundary from '@/components/states/ErrorBoundary';
import ErrorState from '@/components/states/ErrorState';
import LoadingState from '@/components/states/LoadingState';
import DiagnosticsPanel from '@/components/states/DiagnosticsPanel';

const App = () => {
  const [tab, setTab] = useState<ProviderTab>('overview');
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [refreshBusy, setRefreshBusy] = useState(false);
  const [trendBusy, setTrendBusy] = useState(false);

  const widgetMode = useSettingsStore((s) => s.widgetMode);
  const setWidgetMode = useSettingsStore((s) => s.setWidgetMode);
  const demoMode = useSettingsStore((s) => s.demoMode);
  const setDemoMode = useSettingsStore((s) => s.setDemoMode);
  const enabledProviders = useSettingsStore((s) => s.enabledProviders);
  const setProviderEnabled = useSettingsStore((s) => s.setProviderEnabled);
  const refreshCadence = useSettingsStore((s) => s.refreshCadence);
  const setRefreshCadence = useSettingsStore((s) => s.setRefreshCadence);
  const apiKeys = useSettingsStore((s) => s.apiKeys);
  const setApiKeyLocal = useSettingsStore((s) => s.setApiKey);
  const theme = useSettingsStore((s) => s.theme);
  const setTheme = useSettingsStore((s) => s.setTheme);

  const snapshots = useUsageStore((s) => s.snapshots);
  const trends = useUsageStore((s) => s.trends);
  const statuses = useUsageStore((s) => s.statuses);
  const loading = useUsageStore((s) => s.loading);
  const error = useUsageStore((s) => s.error);
  const refreshProvider = useUsageStore((s) => s.refreshProvider);
  const refreshAll = useUsageStore((s) => s.refreshAll);
  const fetchTrend = useUsageStore((s) => s.fetchTrend);
  const setApiKeyRemote = useUsageStore((s) => s.setApiKey);
  const setCadenceRemote = useUsageStore((s) => s.setCadence);

  useUsageData();
  useProviderStatus();
  useGlassTheme(theme);

  const currentProvider = tab === 'overview' ? null : (tab as ProviderId);
  const trendProvider: ProviderId = tab === 'overview' ? 'claude' : (tab as ProviderId);
  const activeProviders = useMemo(
    () => (['claude', 'codex', 'gemini'] as ProviderId[]).filter((provider) => enabledProviders[provider]),
    [enabledProviders]
  );
  const hasAnySnapshot = useMemo(
    () => activeProviders.some((provider) => Boolean(snapshots[provider])),
    [activeProviders, snapshots]
  );

  useEffect(() => {
    if (tab === 'overview') return;
    if (enabledProviders[tab as ProviderId]) return;
    if (activeProviders.length > 0) {
      setTab(activeProviders[0]);
      return;
    }
    setTab('overview');
  }, [tab, enabledProviders, activeProviders]);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'r') {
        event.preventDefault();
        refreshEverything();
        return;
      }
      if ((event.metaKey || event.ctrlKey) && event.key === ',') {
        event.preventDefault();
        setSettingsOpen(true);
        return;
      }
      if (event.key === 'Escape') {
        if (settingsOpen) {
          event.preventDefault();
          setSettingsOpen(false);
        }
        return;
      }
      if (event.key === '1') setTab('claude');
      if (event.key === '2') setTab('codex');
      if (event.key === '3') setTab('gemini');
    };

    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [settingsOpen]);

  const saveKey = async (provider: ProviderId) => {
    const key = apiKeys[provider];
    await setApiKeyRemote(provider, key);
    await refreshProvider(provider);
  };

  const refreshEverything = async () => {
    if (refreshBusy) return;
    setRefreshBusy(true);
    try {
      await refreshAll();
      await Promise.all((['claude', 'codex', 'gemini'] as ProviderId[]).map((provider) => fetchTrend(provider)));
    } catch (error) {
      console.error('refresh all failed', error);
    } finally {
      setRefreshBusy(false);
    }
  };

  const loadTrends = async () => {
    if (trendBusy) return;
    setTrendBusy(true);
    try {
      if (tab === 'overview') {
        await Promise.all((['claude', 'codex', 'gemini'] as ProviderId[]).map((provider) => fetchTrend(provider)));
      } else {
        await fetchTrend(tab as ProviderId);
      }
    } catch (error) {
      console.error('load trends failed', error);
    } finally {
      setTrendBusy(false);
    }
  };

  const renderMain = () => {
    if (loading && !snapshots.claude && !snapshots.codex && !snapshots.gemini) {
      return <LoadingState />;
    }

    if (error) {
      return <ErrorState message={error} onRetry={refreshEverything} />;
    }

    if (activeProviders.length === 0) {
      return <EmptyState onOpenSettings={() => setSettingsOpen(true)} />;
    }

    if (!hasAnySnapshot) {
      return (
        <EmptyState
          onOpenSettings={() => setSettingsOpen(true)}
          title="No usage data yet"
          message="No provider returned usage statistics. Add credentials, verify local CLI logs, or enable Demo mode."
          ctaLabel="Open Settings"
        />
      );
    }

    if (tab === 'overview') {
      return <ProviderOverview snapshots={snapshots} trends={trends} />;
    }

    return (
      <ErrorBoundary onRetry={() => refreshProvider(currentProvider!)}>
        <ProviderCard
          snapshot={snapshots[currentProvider!]}
          trend={trends[currentProvider!]}
          onRefresh={() => {
            refreshProvider(currentProvider!);
            fetchTrend(currentProvider!);
          }}
        />
      </ErrorBoundary>
    );
  };

  if (widgetMode) {
    return <WidgetMode snapshots={snapshots} onExpand={() => setWidgetMode(false)} />;
  }

  return (
    <ErrorBoundary onRetry={refreshEverything}>
      <AppShell
        activeTab={tab}
        demoMode={demoMode}
        onTabChange={(next) => {
          setSettingsOpen(false);
          setTab(next);
          if (next !== 'overview') fetchTrend(next as ProviderId);
        }}
        onOpenSettings={() => setSettingsOpen((v) => !v)}
        onOpenTrends={() => {
          setSettingsOpen(false);
          if (tab === 'overview') {
            setTab('claude');
            fetchTrend('claude');
            return;
          }
          fetchTrend(trendProvider);
        }}
        settingsOpen={settingsOpen}
      >
        <div style={{ display: 'grid', gap: 10 }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
            <button
              className="glass-pill compact-action-btn"
              onClick={refreshEverything}
              style={{ cursor: 'pointer' }}
              disabled={refreshBusy}
              title="Refresh all providers"
            >
              <RefreshCw size={14} className={refreshBusy ? 'spin-icon' : ''} />
            </button>
            <button
              className="glass-pill compact-action-btn"
              onClick={loadTrends}
              style={{ cursor: 'pointer' }}
              disabled={trendBusy}
              title="Load trends"
            >
              <TrendingUp size={14} className={trendBusy ? 'spin-icon' : ''} />
            </button>
          </div>

          {settingsOpen && (
            <GlassPanel style={{ padding: 12, display: 'grid', gap: 8 }}>
              <div className="provider-name">Settings</div>
              <div style={{ display: 'flex', gap: 8, flexWrap: 'wrap' }}>
                <GlassToggle checked={widgetMode} onChange={setWidgetMode} label="Widget" />
                <GlassToggle checked={demoMode} onChange={setDemoMode} label="Demo mode" />
                <select
                  className="glass-pill"
                  value={theme}
                  onChange={(e) => setTheme(e.target.value as 'light' | 'dark' | 'system')}
                >
                  <option value="system">Theme: System</option>
                  <option value="dark">Theme: Dark</option>
                  <option value="light">Theme: Light</option>
                </select>
                <select
                  className="glass-pill"
                  value={refreshCadence}
                  onChange={(e) => {
                    const value = e.target.value as typeof refreshCadence;
                    setRefreshCadence(value);
                    setCadenceRemote(value);
                  }}
                >
                  <option value="manual">Manual</option>
                  <option value="every30s">30s</option>
                  <option value="every1m">1m</option>
                  <option value="every2m">2m</option>
                  <option value="every5m">5m</option>
                  <option value="every15m">15m</option>
                </select>
              </div>

              {(['claude', 'codex', 'gemini'] as ProviderId[]).map((provider) => (
                <div key={provider} style={{ display: 'grid', gap: 6 }}>
                  <GlassToggle
                    checked={enabledProviders[provider]}
                    onChange={(next) => setProviderEnabled(provider, next)}
                    label={`Enable ${provider}`}
                  />
                  <div style={{ display: 'grid', gridTemplateColumns: '1fr auto', gap: 8 }}>
                    <input
                      value={apiKeys[provider]}
                      onChange={(e) => setApiKeyLocal(provider, e.target.value)}
                      className="glass-pill"
                      placeholder={
                        provider === 'claude' ? 'Auto-detected from ~/.claude'
                        : provider === 'codex' ? 'Auto-detected from ~/.codex'
                        : 'Gemini API key'
                      }
                      style={{ width: '100%' }}
                    />
                    <button className="glass-pill" onClick={() => saveKey(provider)} style={{ cursor: 'pointer' }}>
                      Save
                    </button>
                  </div>
                </div>
              ))}

              <DiagnosticsPanel
                statuses={statuses}
                snapshots={snapshots}
                globalError={error}
                demoMode={demoMode}
              />
            </GlassPanel>
          )}

          {renderMain()}
        </div>
      </AppShell>
    </ErrorBoundary>
  );
};

export default App;
