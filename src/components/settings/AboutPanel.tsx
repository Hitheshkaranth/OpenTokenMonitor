import { Activity, Cpu, Github, LayoutDashboard, Package } from 'lucide-react';
import { openUrl } from '@tauri-apps/plugin-opener';
import GlassButton from '@/components/glass/GlassButton';
import GlassPanel from '@/components/glass/GlassPanel';
import { isTauriRuntime } from '@/utils/runtime';

const repoUrl = 'https://github.com/Hitheshkaranth/OpenTokenMonitor';
const appVersion = '0.2.0';

const signalCards = [
  {
    label: 'Providers',
    value: 'Claude, Codex, Gemini',
    icon: Cpu,
  },
  {
    label: 'Surfaces',
    value: 'Widget and dashboard',
    icon: LayoutDashboard,
  },
  {
    label: 'Coverage',
    value: 'Usage, prompts, and health',
    icon: Activity,
  },
  {
    label: 'Version',
    value: `v${appVersion}`,
    icon: Package,
  },
] as const;

const openRepo = async () => {
  try {
    if (isTauriRuntime()) {
      await openUrl(repoUrl);
      return;
    }
  } catch (error) {
    console.error('failed to open repo url', error);
  }

  window.open(repoUrl, '_blank', 'noopener,noreferrer');
};

const AboutPanel = () => (
  <div className="about-shell">
    <GlassPanel className="settings-section about-hero-panel">
      <div className="about-hero-grid">
        <div className="about-brand-block">
          <div className="about-logo-frame">
            <img src="/open_token_monitor_icon.png" alt="OpenToken Monitor" className="about-app-logo" />
          </div>
          <div className="about-brand-copy">
            <div className="settings-section-title">About</div>
            <div className="about-title">OpenToken Monitor</div>
            <span className="glass-pill about-version-pill">Version {appVersion}</span>
            <div className="about-description">
              OpenToken Monitor is a local-first desktop monitor for Claude, Codex, and Gemini usage.
              It brings quota windows, model activity, recent prompts, and provider health into one glass surface.
            </div>
            <div className="about-badge-row">
              <span className="glass-pill about-badge">Local-first</span>
              <span className="glass-pill about-badge">Desktop widget + dashboard</span>
              <span className="glass-pill about-badge">Recent CLI history</span>
            </div>
          </div>
        </div>

        <div className="about-signal-grid">
          {signalCards.map((card) => {
            const Icon = card.icon;

            return (
              <div key={card.label} className="about-signal-card">
                <span className="about-signal-icon">
                  <Icon size={14} strokeWidth={2.2} />
                </span>
                <div className="about-signal-copy">
                  <span className="about-signal-label">{card.label}</span>
                  <span className="about-signal-value">{card.value}</span>
                </div>
              </div>
            );
          })}
        </div>
      </div>
    </GlassPanel>

    <GlassPanel className="settings-section about-detail-panel about-repo-panel">
      <div className="settings-section-title">Project</div>
      <div className="about-maintainer-card">
        <div className="about-maintainer-name">Developed by Hithesh Karanth</div>
        <div className="about-project-copy">
          OpenToken Monitor stays openly available on GitHub, with the repository acting as the canonical home for the project.
        </div>
      </div>
      <div className="about-version-strip">
        <div className="about-version-card">
          <span className="about-version-label">Current version</span>
          <span className="about-version-value">{appVersion}</span>
        </div>
        <div className="about-version-card">
          <span className="about-version-label">Release track</span>
          <span className="about-version-value">Open source</span>
        </div>
      </div>
      <div className="about-action-surface">
        <div className="about-action-copy">
          <div className="about-action-title">Project repository</div>
          <div className="metric-label about-repo-link">github.com/Hitheshkaranth/OpenTokenMonitor</div>
        </div>
        <div className="about-action-row">
          <GlassButton variant="primary" size="sm" onClick={openRepo} style={{ display: 'inline-flex', alignItems: 'center', gap: 6 }}>
            <Github size={12} />
            GitHub Repo
          </GlassButton>
        </div>
      </div>
      <div className="about-footnote">Always open source. Local-first by design.</div>
    </GlassPanel>
  </div>
);

export default AboutPanel;
