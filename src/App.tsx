import { useEffect, useMemo, useState } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { LogicalSize } from '@tauri-apps/api/dpi';
import NavBar from '@/components/layout/Sidebar';
import WidgetMode from '@/components/layout/WidgetMode';
import ProviderCard from '@/components/providers/ProviderCard';
import ProviderOverview from '@/components/providers/ProviderOverview';
import SettingsPage from '@/components/settings/SettingsPage';
import { PageId, ProviderId } from '@/types';
import { useSettingsStore } from '@/stores/settingsStore';
import { useUsageStore } from '@/stores/usageStore';
import { useUsageData } from '@/hooks/useUsageData';
import { useProviderStatus } from '@/hooks/useProviderStatus';
import { useGlassTheme } from '@/hooks/useGlassTheme';
import { isTauriRuntime } from '@/utils/runtime';
import EmptyState from '@/components/states/EmptyState';
import ErrorBoundary from '@/components/states/ErrorBoundary';
import ErrorState from '@/components/states/ErrorState';
import LoadingState from '@/components/states/LoadingState';


const App = () => {
  const [page, setPage] = useState<PageId>('overview');
  const [refreshBusy, setRefreshBusy] = useState(false);

  const widgetMode = useSettingsStore((s) => s.widgetMode);
  const setWidgetMode = useSettingsStore((s) => s.setWidgetMode);
  const enabledProviders = useSettingsStore((s) => s.enabledProviders);
  const theme = useSettingsStore((s) => s.theme);

  const snapshots = useUsageStore((s) => s.snapshots);
  const trends = useUsageStore((s) => s.trends);
  const modelBreakdowns = useUsageStore((s) => s.modelBreakdowns);
  const recentActivity = useUsageStore((s) => s.recentActivity);
  const statuses = useUsageStore((s) => s.statuses);
  const alerts = useUsageStore((s) => s.alerts);
  const loading = useUsageStore((s) => s.loading);
  const error = useUsageStore((s) => s.error);
  const refreshProvider = useUsageStore((s) => s.refreshProvider);
  const refreshAll = useUsageStore((s) => s.refreshAll);
  const fetchTrend = useUsageStore((s) => s.fetchTrend);
  const fetchModelBreakdown = useUsageStore((s) => s.fetchModelBreakdown);
  const fetchRecentActivity = useUsageStore((s) => s.fetchRecentActivity);
  const fetchUsageReport = useUsageStore((s) => s.fetchUsageReport);

  useUsageData();
  useProviderStatus();
  useGlassTheme(theme);

  // Resize window when toggling widget mode
  useEffect(() => {
    if (!isTauriRuntime()) return;
    const win = getCurrentWindow();
    const h = widgetMode ? 182 : 390;
    (async () => {
      try {
        await win.setResizable(true);
        // Clear all size constraints first
        await win.setSizeConstraints({
          minWidth: 360, maxWidth: 360,
          minHeight: 100, maxHeight: 600,
        });
        // Now resize
        await win.setSize(new LogicalSize(360, h));
        // Lock to target
        await win.setSizeConstraints({
          minWidth: 360, maxWidth: 360,
          minHeight: h, maxHeight: h,
        });
        await win.setResizable(false);
      } catch (err) {
        console.error('resize failed', err);
      }
    })();
  }, [widgetMode]);

  const activeProviders = useMemo(
    () => (['claude', 'codex', 'gemini'] as ProviderId[]).filter((p) => enabledProviders[p]),
    [enabledProviders]
  );

  const hasAnySnapshot = useMemo(
    () => activeProviders.some((p) => Boolean(snapshots[p])),
    [activeProviders, snapshots]
  );

  // Eagerly fetch trends for all providers so overview sparklines load fast
  useEffect(() => {
    (['claude', 'codex', 'gemini'] as ProviderId[]).forEach((p) => {
      fetchTrend(p);
      fetchModelBreakdown(p);
      fetchRecentActivity(p);
    });
  }, [fetchModelBreakdown, fetchRecentActivity, fetchTrend]);

  // Also re-fetch when navigating to a specific provider page
  useEffect(() => {
    if (page !== 'overview' && page !== 'settings') {
      fetchTrend(page);
      fetchModelBreakdown(page);
      fetchRecentActivity(page);
    }
  }, [page, fetchModelBreakdown, fetchRecentActivity, fetchTrend]);

  // Auto-fetch usage report when snapshots update
  useEffect(() => {
    if (!activeProviders.some((p) => Boolean(snapshots[p]))) return;
    fetchUsageReport().catch(() => undefined);
  }, [
    activeProviders,
    snapshots.claude?.fetched_at,
    snapshots.codex?.fetched_at,
    snapshots.gemini?.fetched_at,
    fetchUsageReport,
  ]);

  // Redirect to valid page if current provider gets disabled
  useEffect(() => {
    if (page === 'overview' || page === 'settings') return;
    if (enabledProviders[page]) return;
    setPage(activeProviders.length > 0 ? activeProviders[0] : 'overview');
  }, [page, enabledProviders, activeProviders]);

  // Keyboard shortcuts
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement;
      const isInput = target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.tagName === 'SELECT';

      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'r') {
        event.preventDefault();
        refreshEverything();
        return;
      }
      if ((event.metaKey || event.ctrlKey) && event.key === ',') {
        event.preventDefault();
        setPage('settings');
        return;
      }
      if (event.key === 'Escape') {
        setPage('overview');
        return;
      }

      // Single-key shortcuts suppressed when input is focused
      if (isInput) return;
      if (event.key === '1') setPage('claude');
      if (event.key === '2') setPage('codex');
      if (event.key === '3') setPage('gemini');
    };

    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, []);

  const refreshEverything = async () => {
    if (refreshBusy) return;
    setRefreshBusy(true);
    try {
      await refreshAll();
      await Promise.all(
        (['claude', 'codex', 'gemini'] as ProviderId[]).flatMap((p) => [
          fetchTrend(p),
          fetchRecentActivity(p),
        ])
      );
      await fetchUsageReport();
    } catch (err) {
      console.error('refresh all failed', err);
    } finally {
      setRefreshBusy(false);
    }
  };

  const renderContent = () => {
    if (page === 'settings') {
      return <SettingsPage />;
    }

    if (loading && !snapshots.claude && !snapshots.codex && !snapshots.gemini) {
      return <LoadingState />;
    }

    if (error) {
      return <ErrorState message={error} onRetry={refreshEverything} />;
    }

    if (activeProviders.length === 0) {
      return <EmptyState onOpenSettings={() => setPage('settings')} />;
    }

    if (!hasAnySnapshot) {
      return (
        <EmptyState
          onOpenSettings={() => setPage('settings')}
          title="No usage data yet"
          message="No provider returned usage statistics. Add credentials or verify local CLI logs."
          ctaLabel="Open Settings"
        />
      );
    }

    if (page === 'overview') {
      return (
        <ProviderOverview
          snapshots={snapshots}
          trends={trends}
          modelBreakdowns={modelBreakdowns}
          alerts={alerts}
          statuses={statuses}
          onNavigate={(provider) => setPage(provider)}
        />
      );
    }

    const currentProvider = page as ProviderId;
    return (
      <ErrorBoundary onRetry={() => refreshProvider(currentProvider)}>
        <ProviderCard
          snapshot={snapshots[currentProvider]}
          trend={trends[currentProvider]}
          breakdown={modelBreakdowns[currentProvider]}
          recentActivity={recentActivity[currentProvider]}
          alerts={alerts[currentProvider]}
          onRefresh={() => {
            refreshProvider(currentProvider);
            fetchTrend(currentProvider);
            fetchModelBreakdown(currentProvider);
            fetchRecentActivity(currentProvider);
            fetchUsageReport();
          }}
        />
      </ErrorBoundary>
    );
  };

  if (widgetMode) {
    return (
      <WidgetMode
        snapshots={snapshots}
        statuses={statuses}
        modelBreakdowns={modelBreakdowns}
        recentActivity={recentActivity}
        onExpand={() => setWidgetMode(false)}
        onRefresh={refreshEverything}
        refreshBusy={refreshBusy}
      />
    );
  }

  return (
    <ErrorBoundary onRetry={refreshEverything}>
      <div className="app-layout">
        <NavBar
          activePage={page}
          onNavigate={setPage}
          onRefresh={refreshEverything}
          refreshBusy={refreshBusy}
          onWidget={() => setWidgetMode(true)}
        />
        <div className="main-content soft-scroll" style={page === 'overview' ? { display: 'flex', flexDirection: 'column' } : undefined}>
          {renderContent()}
        </div>
      </div>
    </ErrorBoundary>
  );
};

export default App;
