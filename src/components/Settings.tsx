import React, { useState } from 'react';
import { ChevronLeft, Lock, ExternalLink } from 'lucide-react';
import { AppConfig, ProviderName } from '../types';

interface SettingsProps {
  config: AppConfig;
  onSave: (config: AppConfig) => void;
  onBack: () => void;
}

const Settings: React.FC<SettingsProps> = ({ config, onSave, onBack }) => {
  const [localConfig, setLocalConfig] = useState<AppConfig>(config);
  const [saved, setSaved] = useState(false);

  const handleSave = () => {
    onSave(localConfig);
    setSaved(true);
    setTimeout(() => setSaved(false), 2000);
  };

  const updateApiKey = (provider: ProviderName, key: string) => {
    setLocalConfig({
      ...localConfig,
      providers: {
        ...localConfig.providers,
        [provider]: { ...localConfig.providers[provider], apiKey: key }
      }
    });
  };

  const updateUsageAuth = (patch: Partial<NonNullable<AppConfig['usageAuth']>>) => {
    setLocalConfig({
      ...localConfig,
      usageAuth: {
        autoBridge: localConfig.usageAuth?.autoBridge ?? true,
        codexBearerToken: localConfig.usageAuth?.codexBearerToken ?? '',
        claudeCookie: localConfig.usageAuth?.claudeCookie ?? '',
        ...patch,
      },
    });
  };

  return (
    <div className="animate-fade-in" style={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
      <header style={{ padding: '16px', display: 'flex', alignItems: 'center', gap: '12px', borderBottom: '1px solid var(--border-subtle)' }}>
        <button onClick={onBack} style={{ background: 'none', border: 'none', color: 'var(--text-primary)', cursor: 'pointer' }}>
          <ChevronLeft size={20} />
        </button>
        <div>
          <h2 style={{ fontSize: '16px', fontWeight: 600 }}>Settings</h2>
          <p style={{ fontSize: '10px', color: 'var(--text-secondary)' }}>API Configuration & Preferences</p>
        </div>
      </header>

      <div style={{ flex: 1, overflowY: 'auto', padding: '16px' }}>
        <section style={{ marginBottom: '24px' }}>
          <h3 style={{ fontSize: '11px', color: 'var(--text-secondary)', marginBottom: '12px', textTransform: 'uppercase', letterSpacing: '0.5px' }}>API Keys</h3>
          
          {(['anthropic', 'openai', 'google'] as ProviderName[]).map((p) => (
            <div key={p} style={{ marginBottom: '16px' }}>
              <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: '6px', alignItems: 'center' }}>
                <span style={{ fontSize: '12px', fontWeight: 600, textTransform: 'capitalize' }}>{p}</span>
                <a href="#" style={{ fontSize: '10px', color: 'var(--text-secondary)', display: 'flex', alignItems: 'center', gap: '2px', textDecoration: 'none' }}>
                  Get key <ExternalLink size={10} />
                </a>
              </div>
              <input 
                type="password"
                value={localConfig.providers[p].apiKey}
                onChange={(e) => updateApiKey(p, e.target.value)}
                placeholder={`${p === 'anthropic' ? 'sk-ant-...' : p === 'openai' ? 'sk-...' : 'AIza...'}`}
                style={{ 
                  width: '100%', 
                  background: 'rgba(0,0,0,0.2)', 
                  border: '1px solid var(--border-subtle)', 
                  borderRadius: '8px', 
                  padding: '10px', 
                  color: 'white',
                  fontFamily: 'var(--font-mono)',
                  fontSize: '12px'
                }}
              />
            </div>
          ))}
        </section>

        <section style={{ marginBottom: '24px' }}>
          <h3 style={{ fontSize: '11px', color: 'var(--text-secondary)', marginBottom: '12px', textTransform: 'uppercase', letterSpacing: '0.5px' }}>
            Daily Command Limits
          </h3>
          <p style={{ fontSize: '10px', color: 'var(--text-muted)', marginBottom: '12px', lineHeight: '1.5' }}>
            Set a daily target for each CLI tool. The Stats tab shows a progress bar of today's usage vs this limit.
          </p>
          {(['anthropic', 'openai', 'google'] as ProviderName[]).map((p) => (
            <div key={p} style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: '10px' }}>
              <span style={{ fontSize: '12px', fontWeight: 600, textTransform: 'capitalize' }}>
                {p === 'anthropic' ? 'Claude' : p === 'openai' ? 'OpenAI Codex' : 'Gemini'} / day
              </span>
              <input
                type="number"
                min={1}
                max={9999}
                value={localConfig.dailyLimits?.[p] ?? 100}
                onChange={(e) => setLocalConfig({
                  ...localConfig,
                  dailyLimits: { ...(localConfig.dailyLimits ?? { anthropic: 100, openai: 100, google: 100 }), [p]: Number(e.target.value) },
                })}
                style={{
                  width: '72px', textAlign: 'right',
                  background: 'rgba(0,0,0,0.2)',
                  border: '1px solid var(--border-subtle)',
                  borderRadius: '6px', padding: '6px 8px',
                  color: 'white', fontFamily: 'var(--font-mono)', fontSize: '12px',
                }}
              />
            </div>
          ))}
        </section>

        <section style={{ marginBottom: '24px' }}>
          <h3 style={{ fontSize: '11px', color: 'var(--text-secondary)', marginBottom: '12px', textTransform: 'uppercase', letterSpacing: '0.5px' }}>
            Usage Sync (Exact)
          </h3>
          <p style={{ fontSize: '10px', color: 'var(--text-muted)', marginBottom: '12px', lineHeight: '1.5' }}>
            Option 1: auto bridge uses local Codex CLI login token. Option 2: manual secrets let this app read the exact usage pages.
          </p>

          <label style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: '12px' }}>
            <span style={{ fontSize: '12px', fontWeight: 600 }}>Auto bridge (Codex)</span>
            <input
              type="checkbox"
              checked={localConfig.usageAuth?.autoBridge ?? true}
              onChange={(e) => updateUsageAuth({ autoBridge: e.target.checked })}
            />
          </label>

          <div style={{ marginBottom: '12px' }}>
            <div style={{ fontSize: '11px', color: 'var(--text-secondary)', marginBottom: '6px' }}>Manual Codex bearer token (optional override)</div>
            <input
              type="password"
              value={localConfig.usageAuth?.codexBearerToken ?? ''}
              onChange={(e) => updateUsageAuth({ codexBearerToken: e.target.value })}
              placeholder="eyJ... (Bearer token)"
              style={{
                width: '100%',
                background: 'rgba(0,0,0,0.2)',
                border: '1px solid var(--border-subtle)',
                borderRadius: '8px',
                padding: '10px',
                color: 'white',
                fontFamily: 'var(--font-mono)',
                fontSize: '12px',
              }}
            />
          </div>

          <div>
            <div style={{ fontSize: '11px', color: 'var(--text-secondary)', marginBottom: '6px' }}>Manual Claude cookie string</div>
            <input
              type="password"
              value={localConfig.usageAuth?.claudeCookie ?? ''}
              onChange={(e) => updateUsageAuth({ claudeCookie: e.target.value })}
              placeholder="sessionKey=...; ..."
              style={{
                width: '100%',
                background: 'rgba(0,0,0,0.2)',
                border: '1px solid var(--border-subtle)',
                borderRadius: '8px',
                padding: '10px',
                color: 'white',
                fontFamily: 'var(--font-mono)',
                fontSize: '12px',
              }}
            />
          </div>
        </section>

        <section style={{
          background: 'rgba(66, 133, 244, 0.1)',
          border: '1px solid rgba(66, 133, 244, 0.2)',
          borderRadius: '10px',
          padding: '12px',
          display: 'flex',
          gap: '10px'
        }}>
          <Lock size={16} style={{ color: '#4285F4', flexShrink: 0 }} />
          <p style={{ fontSize: '10px', color: 'var(--text-secondary)', lineHeight: '1.4' }}>
            All keys are stored locally using encrypted storage and never leave your device. They are sent directly to the providers' official API endpoints.
          </p>
        </section>
      </div>

      <div style={{ padding: '16px', borderTop: '1px solid var(--border-subtle)' }}>
        <button 
          onClick={handleSave}
          style={{ 
            width: '100%', 
            padding: '12px', 
            borderRadius: '10px', 
            border: 'none', 
            background: saved ? '#22C55E' : 'linear-gradient(135deg, var(--claude-primary), var(--openai-primary), var(--gemini-primary))',
            color: 'white',
            fontWeight: 700,
            fontSize: '13px',
            cursor: 'pointer',
            transition: 'all 0.3s ease'
          }}
        >
          {saved ? '✓ Saved Configuration' : 'Save Changes'}
        </button>
      </div>
    </div>
  );
};

export default Settings;
