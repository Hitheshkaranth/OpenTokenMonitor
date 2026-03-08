import { useState } from 'react';
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

const App = () => {
  const [tab, setTab] = useState<ProviderTab>('overview');
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [refreshBusy, setRefreshBusy] = useState(false);
  const [trendBusy, setTrendBusy] = useState(false);

  const widgetMode = useSettingsStore((s) => s.widgetMode);
  const setWidgetMode = useSettingsStore((s) => s.setWidgetMode);
  const refreshCadence = useSettingsStore((s) => s.refreshCadence);
  const setRefreshCadence = useSettingsStore((s) => s.setRefreshCadence);
  const apiKeys = useSettingsStore((s) => s.apiKeys);
  const setApiKeyLocal = useSettingsStore((s) => s.setApiKey);
  const theme = useSettingsStore((s) => s.theme);
  const setTheme = useSettingsStore((s) => s.setTheme);

  const snapshots = useUsageStore((s) => s.snapshots);
  const trends = useUsageStore((s) => s.trends);
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
    if (tab === 'overview') {
      return <ProviderOverview snapshots={snapshots} trends={trends} />;
    }

    return (
      <ProviderCard
        snapshot={snapshots[currentProvider!]}
        trend={trends[currentProvider!]}
        onRefresh={() => {
          refreshProvider(currentProvider!);
          fetchTrend(currentProvider!);
        }}
      />
    );
  };

  if (widgetMode) {
    return <WidgetMode snapshots={snapshots} onExpand={() => setWidgetMode(false)} />;
  }

  return (
    <AppShell
      activeTab={tab}
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
              <div key={provider} style={{ display: 'grid', gridTemplateColumns: '1fr auto', gap: 8 }}>
                <input
                  value={apiKeys[provider]}
                  onChange={(e) => setApiKeyLocal(provider, e.target.value)}
                  className="glass-pill"
                  placeholder={`${provider} API key / token`}
                  style={{ width: '100%' }}
                />
                <button className="glass-pill" onClick={() => saveKey(provider)} style={{ cursor: 'pointer' }}>
                  Save
                </button>
              </div>
            ))}
          </GlassPanel>
        )}

        {renderMain()}
      </div>
    </AppShell>
  );
};

export default App;
