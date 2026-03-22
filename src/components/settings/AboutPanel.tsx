import { Activity, Cpu, Github, LayoutDashboard, Package } from 'lucide-react';
import { openUrl } from '@tauri-apps/plugin-opener';
import GlassButton from '@/components/glass/GlassButton';
import { APP_NAME, APP_REPO_URL, APP_VERSION } from '@/constants/appMeta';
import { isTauriRuntime } from '@/utils/runtime';

const signalCards = [
  { label: 'Providers', value: 'Claude, Codex, Gemini', icon: Cpu },
  { label: 'Surfaces', value: 'Widget + Dashboard', icon: LayoutDashboard },
  { label: 'Coverage', value: 'Usage, prompts, health', icon: Activity },
  { label: 'Version', value: `v${APP_VERSION}`, icon: Package },
] as const;

const openRepo = async () => {
  try {
    if (isTauriRuntime()) {
      await openUrl(APP_REPO_URL);
      return;
    }
  } catch (error) {
    console.error('failed to open repo url', error);
  }
  window.open(APP_REPO_URL, '_blank', 'noopener,noreferrer');
};

const AboutPanel = () => (
  <div className="abt-root">
    {/* Hero */}
    <div className="abt-hero">
      <div className="abt-logo-frame">
        <img src="/open_token_monitor_icon.png" alt="OpenToken Monitor" className="abt-logo" />
      </div>
      <div className="abt-hero-copy">
        <span className="abt-app-name">{APP_NAME}</span>
        <span className="stg-badge">v{APP_VERSION}</span>
      </div>
      <p className="abt-desc">
        Local-first desktop monitor for Claude, Codex, and Gemini usage.
        Quota windows, model activity, prompts, and provider health in one surface.
      </p>
      <div className="abt-tags">
        <span className="stg-badge">Local-first</span>
        <span className="stg-badge">Widget + Dashboard</span>
        <span className="stg-badge">CLI History</span>
      </div>
    </div>

    {/* Signal cards */}
    <div className="abt-signals">
      {signalCards.map((card) => {
        const Icon = card.icon;
        return (
          <div key={card.label} className="abt-signal">
            <span className="abt-signal-icon"><Icon size={13} strokeWidth={2.2} /></span>
            <span className="abt-signal-label">{card.label}</span>
            <span className="abt-signal-value">{card.value}</span>
          </div>
        );
      })}
    </div>

    {/* Project info */}
    <div className="abt-project-card">
      <span className="abt-section-title">Project</span>
      <span className="abt-maintainer">Developed by Hithesh Karanth</span>
      <p className="abt-project-desc">
        Open source on GitHub. Local-first by design.
      </p>
      <div className="abt-version-row">
        <div className="abt-version-cell">
          <span className="abt-version-label">Version</span>
          <span className="abt-version-value">{APP_VERSION}</span>
        </div>
        <div className="abt-version-cell">
          <span className="abt-version-label">Track</span>
          <span className="abt-version-value">Open source</span>
        </div>
      </div>
      <GlassButton variant="primary" size="sm" onClick={openRepo} style={{ display: 'inline-flex', alignItems: 'center', gap: 6, alignSelf: 'center' }}>
        <Github size={12} />
        GitHub Repo
      </GlassButton>
    </div>
  </div>
);

export default AboutPanel;
