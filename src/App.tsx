import { invoke } from '@tauri-apps/api/core';
import { useEffect, useMemo, useState } from 'react';
import NavBar from '@/components/layout/Sidebar';
import WidgetMode from '@/components/layout/WidgetMode';
import ProjectOverview from '@/components/projects/ProjectOverview';
import ProviderCard from '@/components/providers/ProviderCard';
import ProviderOverview from '@/components/providers/ProviderOverview';
import ComparisonView from '@/components/comparison/ComparisonView';
import PeriodSelector from '@/components/overview/PeriodSelector';
import SettingsPage from '@/components/settings/SettingsPage';
import { PageId, ProviderId } from '@/types';
import { useSettingsStore } from '@/stores/settingsStore';
import { useUsageStore } from '@/stores/usageStore';
import { useUsageData } from '@/hooks/useUsageData';
import { useProviderStatus } from '@/hooks/useProviderStatus';
import { useGlassTheme } from '@/hooks/useGlassTheme';
import { useKeyboardShortcuts } from '@/hooks/useKeyboardShortcuts';
import { useWidgetResize } from '@/hooks/useWidgetResize';
import { useLaunchAtStartupSync } from '@/hooks/useLaunchAtStartupSync';
import EmptyState from '@/components/states/EmptyState';
import ErrorBoundary from '@/components/states/ErrorBoundary';
import ErrorState from '@/components/states/ErrorState';
import LoadingState from '@/components/states/LoadingState';
import { UpdateChecker } from '@/components/system/UpdateChecker';

// App is the frontend orchestration layer. It does not own provider-specific
// parsing logic; instead it coordinates Zustand stores, startup hooks, window
// behavior, and which major surface should be rendered.
const App = () => {
  const [page, setPage] = useState<PageId>('overview');
  const [refreshBusy, setRefreshBusy] = useState(false);

  const widgetMode = useSettingsStore((s) => s.widgetMode);
  const setWidgetMode = useSettingsStore((s) => s.setWidgetMode);
  const enabledProviders = useSettingsStore((s) => s.enabledProviders);
  const theme = useSettingsStore((s) => s.theme);
  const launchAtStartup = useSettingsStore((s) => s.launchAtStartup);
  const settingsHydrated = useSettingsStore((s) => s.hydrated);
  const trendDays = useSettingsStore((s) => s.resolveTrendDays());
  const notificationsEnabled = useSettingsStore((s) => s.notificationsEnabled);
  const trayTitleMode = useSettingsStore((s) => s.trayTitleMode);

  const isProviderPage = (target: PageId): target is ProviderId =>
    target === 'claude' || target === 'codex' || target === 'antigravity';

  const snapshots = useUsageStore((s) => s.snapshots);
  const costHistory = useUsageStore((s) => s.costHistory);
  const trends = useUsageStore((s) => s.trends);
  const modelBreakdowns = useUsageStore((s) => s.modelBreakdowns);
  const recentActivity = useUsageStore((s) => s.recentActivity);
  const statuses = useUsageStore((s) => s.statuses);
  const authStates = useUsageStore((s) => s.authStates);
  const alerts = useUsageStore((s) => s.alerts);
  const projectUsage = useUsageStore((s) => s.projectUsage);
  const sessionUsage = useUsageStore((s) => s.sessionUsage);
  const loading = useUsageStore((s) => s.loading);
  const error = useUsageStore((s) => s.error);
  const refreshProvider = useUsageStore((s) => s.refreshProvider);
  const refreshAll = useUsageStore((s) => s.refreshAll);
  const fetchCostHistory = useUsageStore((s) => s.fetchCostHistory);
  const fetchTrend = useUsageStore((s) => s.fetchTrend);
  const fetchModelBreakdown = useUsageStore((s) => s.fetchModelBreakdown);
  const fetchRecentActivity = useUsageStore((s) => s.fetchRecentActivity);
  const fetchUsageReport = useUsageStore((s) => s.fetchUsageReport);
  const fetchForecasts = useUsageStore((s) => s.fetchForecasts);
  const fetchProjectUsage = useUsageStore((s) => s.fetchProjectUsage);
  const fetchSessionUsage = useUsageStore((s) => s.fetchSessionUsage);
  const fetchProviderProjectUsage = useUsageStore((s) => s.fetchProviderProjectUsage);

  // These hooks establish the long-lived runtime behavior for the desktop app:
  // hydrate usage data, poll provider status, apply theme, sync OS-level
  // settings, and resize the window when widget mode toggles.
  useUsageData();
  useProviderStatus();
  useGlassTheme(theme);
  useLaunchAtStartupSync(launchAtStartup, settingsHydrated);
  useWidgetResize(widgetMode);

  const activeProviders = useMemo(
    () => (['claude', 'codex', 'antigravity'] as ProviderId[]).filter((p) => enabledProviders[p]),
    [enabledProviders]
  );

  // The dashboard only needs one successful snapshot to become useful; providers
  // that are still retrying can keep rendering as partial/unavailable states.
  const hasAnySnapshot = useMemo(
    () => activeProviders.some((p) => Boolean(snapshots[p])),
    [activeProviders, snapshots]
  );

  // Eagerly fetch trends for all providers so overview sparklines load fast
  useEffect(() => {
    (['claude', 'codex', 'antigravity'] as ProviderId[]).forEach((p) => {
      fetchCostHistory(p, trendDays);
      fetchTrend(p, trendDays);
      fetchModelBreakdown(p, trendDays);
      fetchRecentActivity(p, 120);
    });
  }, [fetchCostHistory, fetchModelBreakdown, fetchRecentActivity, fetchTrend, trendDays]);

  // Also re-fetch when navigating to a specific provider page
  useEffect(() => {
    if (!isProviderPage(page)) return;
    fetchCostHistory(page, trendDays);
    fetchTrend(page, trendDays);
    fetchModelBreakdown(page, trendDays);
    fetchRecentActivity(page, 120);
  }, [page, fetchCostHistory, fetchModelBreakdown, fetchRecentActivity, fetchTrend, trendDays]);

  // Auto-fetch usage report and time-to-limit projections when snapshots update
  useEffect(() => {
    if (!activeProviders.some((p) => Boolean(snapshots[p]))) return;
    fetchUsageReport(trendDays).catch(() => undefined);
    fetchForecasts();
  }, [
    activeProviders,
    snapshots.claude?.fetched_at,
    snapshots.codex?.fetched_at,
    snapshots.antigravity?.fetched_at,
    fetchUsageReport,
    fetchForecasts,
    trendDays,
  ]);

  // Exact per-project / per-session usage for the Projects page and the
  // provider detail page, kept fresh as new snapshots arrive.
  const latestFetch = [snapshots.claude?.fetched_at, snapshots.codex?.fetched_at, snapshots.antigravity?.fetched_at].join('|');
  useEffect(() => {
    if (page === 'projects') {
      fetchProjectUsage(trendDays);
      fetchSessionUsage(trendDays);
    } else if (isProviderPage(page)) {
      fetchProviderProjectUsage(page, trendDays);
    }
  }, [page, trendDays, latestFetch, fetchProjectUsage, fetchSessionUsage, fetchProviderProjectUsage]);

  // Redirect to valid page if current provider gets disabled
  useEffect(() => {
    if (page === 'overview' || page === 'settings' || page === 'projects' || !isProviderPage(page)) return;
    if (enabledProviders[page]) return;
    setPage(activeProviders.length > 0 ? activeProviders[0] : 'overview');
  }, [page, enabledProviders, activeProviders]);

  // Re-apply persisted per-provider thresholds + cadences after settings
  // hydrate, so background polling and report alerts honor the user's custom
  // ladder from the moment the app launches.
  useEffect(() => {
    if (!settingsHydrated) return;
    const settings = useSettingsStore.getState();
    (['claude', 'codex', 'antigravity'] as ProviderId[]).forEach((provider) => {
      const thresholds = settings.perProviderThresholds[provider];
      invoke('set_thresholds', {
        provider,
        warning: thresholds.warning,
        high: thresholds.high,
        critical: thresholds.critical,
      }).catch(() => undefined);
      invoke('set_per_provider_cadence', {
        provider,
        cadence: settings.perProviderCadence[provider],
      }).catch(() => undefined);
    });
  }, [settingsHydrated]);

  // Desktop notifications are raised by the backend after every refresh (so
  // they work while the window is hidden); the UI only forwards the switch.
  useEffect(() => {
    if (!settingsHydrated) return;
    invoke('set_notifications_enabled', { enabled: notificationsEnabled }).catch(() => undefined);
  }, [settingsHydrated, notificationsEnabled]);

  useEffect(() => {
    if (!settingsHydrated) return;
    invoke('set_tray_title_mode', { mode: trayTitleMode }).catch(() => undefined);
  }, [settingsHydrated, trayTitleMode]);

  const refreshEverything = async () => {
    if (refreshBusy) return;
    setRefreshBusy(true);
    try {
      // Refresh snapshots first, then hydrate the derived surfaces that depend on
      // the latest provider state.
      await refreshAll();
      (await Promise.all(
        (['claude', 'codex', 'antigravity'] as ProviderId[]).flatMap((p) => [
          fetchCostHistory(p, trendDays),
          fetchTrend(p, trendDays),
          fetchRecentActivity(p, 120),
        ])
      ));
      await fetchUsageReport(trendDays);
    } catch (err) {
      console.error('refresh all failed', err);
    } finally {
      setRefreshBusy(false);
    }
  };

  useKeyboardShortcuts(setPage, refreshEverything);

  // Keep the render branching in one place so widget mode, settings, overview,
  // and provider detail screens all share the same loading/error rules.
  const renderContent = () => {
    if (page === 'settings') {
      return <SettingsPage />;
    }

    if (loading && !snapshots.claude && !snapshots.codex && !snapshots.antigravity) {
      return <LoadingState />;
    }

    if (error && !hasAnySnapshot) {
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

    if (page === 'projects') {
      return (
        <ProjectOverview
          recentActivity={recentActivity}
          costHistory={costHistory}
          projectUsage={projectUsage}
          sessionUsage={sessionUsage}
          periodDays={trendDays}
        />
      );
    }

    if (page === 'comparison') {
      return (
        <ComparisonView
          snapshots={snapshots}
          costHistory={costHistory}
          modelBreakdowns={modelBreakdowns}
          alerts={alerts}
          statuses={statuses}
          trendDays={trendDays}
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
          allRecentActivity={recentActivity}
          costHistory={costHistory}
          alerts={alerts[currentProvider]}
          status={statuses[currentProvider]}
          authState={authStates[currentProvider]}
          onRefresh={() => {
            refreshProvider(currentProvider);
            fetchCostHistory(currentProvider, trendDays);
            fetchTrend(currentProvider, trendDays);
            fetchModelBreakdown(currentProvider, trendDays);
            fetchRecentActivity(currentProvider, 120);
            fetchUsageReport(trendDays);
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
        <UpdateChecker />
        <NavBar
          activePage={page}
          onNavigate={setPage}
          onRefresh={refreshEverything}
          refreshBusy={refreshBusy}
          onWidget={() => setWidgetMode(true)}
        />
        <div className="app-header">
          <PeriodSelector />
        </div>
        <div className="main-content soft-scroll" style={page === 'overview' ? { display: 'flex', flexDirection: 'column', overflow: 'hidden' } : undefined}>
          {renderContent()}
        </div>
      </div>
    </ErrorBoundary>
  );
};

export default App;
