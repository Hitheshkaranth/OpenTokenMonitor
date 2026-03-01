import React, { useEffect, useRef, useState } from 'react';
import { RefreshCw, Settings as SettingsIcon, Activity, PieChart, Terminal, Minus, House } from 'lucide-react';
import { load } from '@tauri-apps/plugin-store';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import {
  AppConfig,
  UsageData,
  CliActivity,
  GeminiStats,
  CliSummaryStats,
  ClaudeUsageCache,
  UsageWindows,
  ExactUsageLimits,
  UsageAuthConfig,
} from './types';
import { fetchAnthropicUsage, fetchOpenAIUsage, fetchGoogleUsage } from './services/api';
import { buildLocalLimitFallback, fetchExactUsageLimits } from './services/usageExact';
import Settings from './components/Settings';
import UsageChart from './components/UsageChart';
import CliActivityItem from './components/CliActivityItem';
import ProviderCard from './components/ProviderCard';
import HomeLimits from './components/HomeLimits';

const DEFAULT_CONFIG: AppConfig = {
  providers: {
    anthropic: { apiKey: '', enabled: true },
    openai: { apiKey: '', enabled: true },
    google: { apiKey: '', enabled: true },
  },
  refreshInterval: 300,
  usageHistory: [],
  theme: 'dark',
  dailyLimits: {
    anthropic: 100,
    openai: 100,
    google: 100,
  },
  usageAuth: {
    autoBridge: true,
    codexBearerToken: '',
    claudeCookie: '',
  },
};

const EMPTY_STATS: CliSummaryStats = {
  total_commands: 0,
  commands_today: 0,
  unique_sessions: 0,
  unique_projects: 0,
  projects: [],
};

const EMPTY_CACHE: ClaudeUsageCache = {
  total_messages: 0,
  total_sessions: 0,
  messages_today: 0,
  sessions_today: 0,
  tokens_today: 0,
  tokens_total: 0,
};

const EMPTY_WINDOWS: UsageWindows = {
  anthropic: { last_4h: 0, last_7d: 0 },
  openai: { last_4h: 0, last_7d: 0 },
  google: { last_4h: 0, last_7d: 0 },
};

const DEFAULT_USAGE_AUTH: UsageAuthConfig = {
  autoBridge: true,
  codexBearerToken: '',
  claudeCookie: '',
};

type MainView = 'home' | 'live' | 'stats' | 'trends';

const App: React.FC = () => {
  const [config, setConfig] = useState<AppConfig>(DEFAULT_CONFIG);
  const [usage, setUsage] = useState<UsageData[]>([]);
  const [cliHistory, setCliHistory] = useState<CliActivity[]>([]);
  const [codexHistory, setCodexHistory] = useState<CliActivity[]>([]);
  const [geminiStats, setGeminiStats] = useState<GeminiStats>({ project_count: 0, session_count: 0, projects: [] });
  const [claudeStats, setClaudeStats] = useState<CliSummaryStats>(EMPTY_STATS);
  const [codexStats, setCodexStats] = useState<CliSummaryStats>(EMPTY_STATS);
  const [claudeCache, setClaudeCache] = useState<ClaudeUsageCache>(EMPTY_CACHE);
  const [exactLimits, setExactLimits] = useState<ExactUsageLimits>(buildLocalLimitFallback(EMPTY_WINDOWS));
  const [view, setView] = useState<'dashboard' | 'settings'>('dashboard');
  const [mainView, setMainView] = useState<MainView>('home');
  const [loading, setLoading] = useState(false);

  const headerRef = useRef<HTMLElement>(null);
  const refreshTimerRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const usageAuthRef = useRef<UsageAuthConfig>(DEFAULT_USAGE_AUTH);

  useEffect(() => {
    const el = headerRef.current;
    if (!el) return;

    const onMouseDown = (e: MouseEvent) => {
      if ((e.target as Element).closest('button')) return;
      getCurrentWindow().startDragging();
    };

    el.addEventListener('mousedown', onMouseDown);
    return () => el.removeEventListener('mousedown', onMouseDown);
  }, []);

  useEffect(() => {
    usageAuthRef.current = config.usageAuth ?? DEFAULT_USAGE_AUTH;
  }, [config.usageAuth]);

  useEffect(() => {
    const init = async () => {
      let currentConfig = DEFAULT_CONFIG;
      try {
        const store = await load('config.json', { defaults: { config: DEFAULT_CONFIG } });
        const saved = await store.get<AppConfig>('config');
        if (saved) {
          currentConfig = normalizeConfig(saved);
          setConfig(currentConfig);
        }
      } catch (e) {
        console.warn('Store unavailable, using defaults:', e);
      }

      await refreshData(currentConfig);
      await refreshUsageWindows(currentConfig.usageAuth ?? DEFAULT_USAGE_AUTH);

      try { setCliHistory(await invoke<CliActivity[]>('get_claude_history')); } catch (e) { console.warn('Could not load Claude history:', e); }
      try { setCodexHistory(await invoke<CliActivity[]>('get_codex_history')); } catch (e) { console.warn('Could not load Codex history:', e); }
      try { setGeminiStats(await invoke<GeminiStats>('get_gemini_stats')); } catch (e) { console.warn('Could not load Gemini stats:', e); }
      try { setClaudeStats(await invoke<CliSummaryStats>('get_claude_stats')); } catch (e) { console.warn('Could not load Claude stats:', e); }
      try { setCodexStats(await invoke<CliSummaryStats>('get_codex_stats')); } catch (e) { console.warn('Could not load Codex stats:', e); }
      try { setClaudeCache(await invoke<ClaudeUsageCache>('get_claude_usage_cache')); } catch (e) { console.warn('Could not load Claude usage cache:', e); }
    };

    init();

    const unlisten = listen<CliActivity>('cli-activity', (event) => {
      const act = event.payload;
      if (act.provider === 'openai') setCodexHistory((prev) => [act, ...prev].slice(0, 50));
      else setCliHistory((prev) => [act, ...prev].slice(0, 50));
      refreshUsageWindows(usageAuthRef.current);
    });

    const unlistenGemini = listen<GeminiStats>('gemini-stats', (event) => {
      setGeminiStats(event.payload);
      refreshUsageWindows(usageAuthRef.current);
    });

    const cliPoll = setInterval(async () => {
      try { setCliHistory(await invoke<CliActivity[]>('get_claude_history')); } catch {}
      try { setCodexHistory(await invoke<CliActivity[]>('get_codex_history')); } catch {}
      try { setGeminiStats(await invoke<GeminiStats>('get_gemini_stats')); } catch {}
      try { setClaudeStats(await invoke<CliSummaryStats>('get_claude_stats')); } catch {}
      try { setCodexStats(await invoke<CliSummaryStats>('get_codex_stats')); } catch {}
      try { setClaudeCache(await invoke<ClaudeUsageCache>('get_claude_usage_cache')); } catch {}
      await refreshUsageWindows(usageAuthRef.current);
    }, 30_000);

    refreshTimerRef.current = setInterval(() => {
      setConfig((cfg) => {
        refreshData(cfg);
        return cfg;
      });
    }, (config.refreshInterval || 300) * 1_000);

    return () => {
      unlisten.then((f) => f());
      unlistenGemini.then((f) => f());
      clearInterval(cliPoll);
      if (refreshTimerRef.current) clearInterval(refreshTimerRef.current);
    };
  }, []);

  const refreshData = async (cfg: AppConfig) => {
    setLoading(true);
    const results = await Promise.all([
      fetchAnthropicUsage(cfg.providers.anthropic.apiKey),
      fetchOpenAIUsage(cfg.providers.openai.apiKey),
      fetchGoogleUsage(cfg.providers.google.apiKey),
    ]);
    setUsage(results);
    setLoading(false);
  };

  const refreshUsageWindows = async (authCfg: UsageAuthConfig) => {
    try {
      const windows = await invoke<UsageWindows>('get_usage_windows');
      const exact = await fetchExactUsageLimits(authCfg, windows);
      setExactLimits(exact);
    } catch (e) {
      console.warn('Could not load usage windows:', e);
    }
  };

  const saveConfig = async (newConfig: AppConfig) => {
    const normalized = normalizeConfig(newConfig);
    const store = await load('config.json', { defaults: { config: DEFAULT_CONFIG } });
    await store.set('config', normalized);
    setConfig(normalized);
    refreshData(normalized);
    refreshUsageWindows(normalized.usageAuth ?? DEFAULT_USAGE_AUTH);
  };

  if (view === 'settings') {
    return (
      <div className="liquid-glass-container">
        <Settings config={config} onSave={saveConfig} onBack={() => setView('dashboard')} />
      </div>
    );
  }

  const totalCost = usage.reduce((acc, u) => acc + (u.totalCost || 0), 0);
  const totalTokens = usage.reduce((acc, u) => acc + (u.totalTokensUsed || 0), 0);
  const claudeToday = claudeCache.messages_today > 0 ? claudeCache.messages_today : claudeStats.commands_today;
  const codexToday = codexStats.commands_today;
  const geminiToday = geminiStats.session_count;
  const allActivity = [...cliHistory, ...codexHistory].sort((a, b) => b.timestamp - a.timestamp).slice(0, 40);

  return (
    <div className="liquid-glass-container animate-fade-in">
      <header
        ref={headerRef}
        data-tauri-drag-region
        style={{
          height: '44px',
          padding: '0 14px',
          flexShrink: 0,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          borderBottom: '1px solid var(--border-subtle)',
          cursor: 'grab',
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: '8px', pointerEvents: 'none' }}>
          <img src="/open_token_monitor_icon.png" width={20} height={20} style={{ borderRadius: '5px' }} alt="OpenTokenMonitor" />
          <span style={{ fontSize: '13px', fontWeight: 700, letterSpacing: '0.01em' }}>OpenTokenMonitor</span>
        </div>

        <div style={{ display: 'flex', gap: '6px' }}>
          <button className="icon-btn" onClick={() => { refreshData(config); refreshUsageWindows(config.usageAuth ?? DEFAULT_USAGE_AUTH); }} title="Refresh">
            <RefreshCw size={13} className={loading ? 'animate-spin' : ''} />
          </button>
          <button className="icon-btn" onClick={() => setView('settings')} title="Settings">
            <SettingsIcon size={13} />
          </button>
          <button className="icon-btn" onClick={() => getCurrentWindow().hide()} title="Hide to tray">
            <Minus size={13} />
          </button>
        </div>
      </header>

      <div style={{
        display: 'grid',
        gridTemplateColumns: 'repeat(2, 1fr)',
        height: '44px',
        flexShrink: 0,
        borderBottom: '1px solid var(--border-subtle)',
        background: 'var(--bg-panel)',
      }}>
        <div className="stat-cell">
          <span className="stat-cell__label">TOTAL COST</span>
          <span className="stat-cell__value">{totalCost > 0 ? `$${totalCost.toFixed(2)}` : '--'}</span>
        </div>
        <div className="stat-cell" style={{ borderLeft: '1px solid var(--border-subtle)' }}>
          <span className="stat-cell__label">{totalTokens > 0 ? 'TOKENS' : 'CLI CMDS'}</span>
          <span className="stat-cell__value">{totalTokens > 0 ? formatTokens(totalTokens) : String(claudeStats.total_commands + codexStats.total_commands)}</span>
        </div>
      </div>

      <div className="main-nav-strip">
        <button className={`tab-btn${mainView === 'home' ? ' tab-btn--active' : ''}`} onClick={() => setMainView('home')}><House size={12} /> Home</button>
        <button className={`tab-btn${mainView === 'live' ? ' tab-btn--active' : ''}`} onClick={() => setMainView('live')}><Terminal size={12} /> Live</button>
        <button className={`tab-btn${mainView === 'stats' ? ' tab-btn--active' : ''}`} onClick={() => setMainView('stats')}><Activity size={12} /> Stats</button>
        <button className={`tab-btn${mainView === 'trends' ? ' tab-btn--active' : ''}`} onClick={() => setMainView('trends')}><PieChart size={12} /> Trends</button>
      </div>

      <main style={{ flex: 1, overflowY: mainView === 'home' ? 'hidden' : 'auto', padding: '10px 12px 12px', minHeight: 0 }}>
        {mainView === 'home' && (
          <HomeLimits limits={exactLimits} />
        )}

        {mainView === 'live' && (
          <section className="home-section">
            <div className="home-section__title"><Terminal size={12} /> Live Activity</div>
            <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
              {allActivity.length > 0 ? (
                allActivity.map((item, i) => <CliActivityItem key={i} item={item} />)
              ) : (
                <div style={{ textAlign: 'center', padding: '28px 20px', color: 'var(--text-secondary)' }}>
                  <Terminal size={28} style={{ marginBottom: '10px', opacity: 0.2 }} />
                  <p style={{ fontSize: '12px' }}>Waiting for CLI activity...</p>
                  <p style={{ fontSize: '10px', marginTop: '4px' }}>
                    Run <code style={{ fontFamily: 'var(--font-mono)' }}>claude</code> or <code style={{ fontFamily: 'var(--font-mono)' }}>codex</code> to see logs.
                  </p>
                </div>
              )}
            </div>
          </section>
        )}

        {mainView === 'stats' && (
          <section className="home-section">
            <div className="home-section__title"><Activity size={12} /> Stats</div>
            <div style={{ display: 'flex', flexDirection: 'column', gap: '10px' }}>
              {usage.map((u, i) => (
                <ProviderCard
                  key={u.provider}
                  data={u}
                  delay={`${i * 0.08}s`}
                  onSettings={() => setView('settings')}
                  dailyLimit={config.dailyLimits?.[u.provider] ?? 100}
                  cliStats={
                    u.provider === 'anthropic'
                      ? {
                          commands: claudeStats.total_commands,
                          commandsToday: claudeStats.commands_today,
                          uniqueSessions: claudeStats.unique_sessions,
                          uniqueProjects: claudeStats.unique_projects,
                          projects: claudeStats.projects,
                        }
                      : u.provider === 'openai'
                        ? {
                            commands: codexStats.total_commands,
                            commandsToday: codexStats.commands_today,
                            uniqueSessions: codexStats.unique_sessions,
                            uniqueProjects: 0,
                            projects: [],
                          }
                        : undefined
                  }
                  claudeCache={u.provider === 'anthropic' ? claudeCache : undefined}
                  geminiStats={u.provider === 'google' ? geminiStats : undefined}
                />
              ))}
            </div>
          </section>
        )}

        {mainView === 'trends' && (
          <section className="home-section">
            <div className="home-section__title"><PieChart size={12} /> Trends</div>
            <UsageChart
              claudeHistory={cliHistory}
              codexHistory={codexHistory}
              geminiStats={geminiStats}
              dailyLimits={config.dailyLimits}
              todayUsage={{
                anthropic: claudeToday,
                openai: codexToday,
                google: geminiToday,
              }}
            />
          </section>
        )}
      </main>

      <footer
        style={{
          height: '32px',
          padding: '0 16px',
          flexShrink: 0,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          fontSize: '9px',
          color: 'var(--text-secondary)',
          borderTop: '1px solid var(--border-subtle)',
          background: 'rgba(0,0,0,0.2)',
        }}
      >
        <span>LIMIT HOME + QUICK NAV</span>
        <div style={{ display: 'flex', gap: '4px' }}>
          <div style={{ width: '6px', height: '6px', borderRadius: '50%', background: 'var(--claude-primary)' }} />
          <div style={{ width: '6px', height: '6px', borderRadius: '50%', background: 'var(--openai-primary)' }} />
          <div style={{ width: '6px', height: '6px', borderRadius: '50%', background: 'var(--gemini-primary)' }} />
        </div>
      </footer>
    </div>
  );
};

const formatTokens = (n: number): string => {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return String(n);
};

export default App;

function normalizeConfig(input: AppConfig): AppConfig {
  return {
    ...DEFAULT_CONFIG,
    ...input,
    dailyLimits: {
      ...DEFAULT_CONFIG.dailyLimits,
      ...(input.dailyLimits ?? {}),
    },
    usageAuth: {
      ...DEFAULT_USAGE_AUTH,
      ...(input.usageAuth ?? {}),
    },
    providers: {
      ...DEFAULT_CONFIG.providers,
      ...(input.providers ?? {}),
    },
  };
}
