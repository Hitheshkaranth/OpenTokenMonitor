import { useMemo } from 'react';
import ProviderLogo from '@/components/providers/ProviderLogo';
import {
  CostEntry,
  ProjectSummary,
  ProviderId,
  RecentActivityEntry,
} from '@/types';
import { buildProjectSummaries } from '@/utils/projectActivity';

type ProjectOverviewProps = {
  recentActivity: Record<ProviderId, RecentActivityEntry[]>;
  costHistory: Record<ProviderId, CostEntry[]>;
};

const formatAge = (timestamp: string) => {
  const deltaMs = Date.now() - new Date(timestamp).getTime();
  if (!Number.isFinite(deltaMs) || deltaMs < 0) return 'now';
  const minutes = Math.floor(deltaMs / 60_000);
  if (minutes < 1) return 'now';
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  return `${days}d ago`;
};

const formatTokens = (value: number) => {
  if (value >= 1_000_000) return `${(value / 1_000_000).toFixed(1)}M`;
  if (value >= 1_000) return `${(value / 1_000).toFixed(1)}K`;
  return String(value);
};

const commandLabel = (prompt: string) => {
  const compact = prompt.replace(/\s+/g, ' ').trim();
  return compact.length > 72 ? `${compact.slice(0, 69)}...` : compact;
};

const providerColors: Record<ProviderId, string> = {
  claude: '#d97757',
  codex: '#10a37f',
  gemini: '#4285f4',
};

const providerAccent: Record<ProviderId, string> = {
  claude: '217 119 87',
  codex: '16 163 127',
  gemini: '66 133 244',
};

const ProjectCard = ({ project }: { project: ProjectSummary }) => {
  const primaryProvider = project.providers[0];

  return (
    <div
      className="proj-card"
      style={{ '--widget-accent': providerAccent[primaryProvider] ?? '148 163 184' } as React.CSSProperties}
    >
      {/* Header: title + cost */}
      <div className="proj-card-header">
        <div className="proj-card-title-col">
          <div className="proj-card-title-row">
            <span className="proj-card-title">{project.label}</span>
            <span className="proj-card-age">{formatAge(project.latest_timestamp)}</span>
          </div>
          <span className="proj-card-path" title={project.path ?? project.label}>
            {project.path ?? 'Terminal/session activity'}
          </span>
        </div>
        <div className="proj-card-cost-col">
          <span className="proj-card-cost">${project.estimated_cost_usd.toFixed(2)}</span>
          <span className="proj-card-cost-sub">today ${project.estimated_cost_today_usd.toFixed(2)}</span>
        </div>
      </div>

      {/* Meta chips: providers, stats, models */}
      <div className="proj-card-chips">
        {project.providers.map((provider) => (
          <span
            key={`${project.id}-${provider}`}
            className="proj-chip proj-chip-provider"
            style={{ borderColor: providerColors[provider], color: providerColors[provider] }}
          >
            <ProviderLogo provider={provider} size={10} />
          </span>
        ))}
        <span className="proj-chip">{project.activity_count} cmds</span>
        <span className="proj-chip">{formatTokens(project.estimated_tokens)} tok</span>
        {project.models.slice(0, 2).map((model) => (
          <span key={`${project.id}-${model}`} className="proj-chip proj-chip-model" title={model}>
            {model}
          </span>
        ))}
        {project.models.length > 2 && (
          <span className="proj-chip">+{project.models.length - 2}</span>
        )}
      </div>

      {/* Command timeline */}
      <div className="proj-card-timeline">
        {project.commands.map((command, index) => (
          <div key={`${project.id}-${command.timestamp}-${index}`} className="proj-cmd">
            <span
              className="proj-cmd-dot"
              style={{ background: providerColors[command.provider] }}
            />
            <div className="proj-cmd-body">
              <div className="proj-cmd-head">
                <span className="proj-cmd-model">{command.model ?? 'unknown'}</span>
              </div>
              <div className="proj-cmd-text" title={command.prompt}>
                {commandLabel(command.prompt)}
              </div>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
};

const ProjectOverview = ({ recentActivity, costHistory }: ProjectOverviewProps) => {
  const projects = useMemo(
    () =>
      buildProjectSummaries(recentActivity, costHistory, {
        maxProjects: 6,
        maxCommandsPerProject: 3,
      }),
    [costHistory, recentActivity]
  );

  return (
    <div className="proj-root">
      {/* Header bar */}
      <div className="proj-header">
        <div className="proj-header-left">
          <span className="proj-header-title">Projects</span>
          <span className="proj-header-subtitle">Cross-model activity &amp; estimated cost</span>
        </div>
        <span className="proj-header-badge">{projects.length} active</span>
      </div>

      {/* Cards */}
      {projects.length === 0 ? (
        <div className="proj-empty">
          No project activity detected yet. Use a provider CLI or workspace to generate activity.
        </div>
      ) : (
        <div className="proj-grid soft-scroll">
          {projects.map((project) => (
            <ProjectCard key={project.id} project={project} />
          ))}
        </div>
      )}
    </div>
  );
};

export default ProjectOverview;
