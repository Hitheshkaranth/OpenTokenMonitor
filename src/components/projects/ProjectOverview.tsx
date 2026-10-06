import { useMemo, useState } from 'react';
import { GlassPill } from '@/components/glass';
import ProviderLogo from '@/components/providers/ProviderLogo';
import {
  CostEntry,
  ProjectCommandEntry,
  ProjectSummary,
  ProjectUsage,
  ProviderId,
  RecentActivityEntry,
  SessionUsage,
} from '@/types';
import { buildProjectSummaries } from '@/utils/projectActivity';

type ProjectOverviewProps = {
  recentActivity: Record<ProviderId, RecentActivityEntry[]>;
  costHistory: Record<ProviderId, CostEntry[]>;
  projectUsage: ProjectUsage[];
  sessionUsage: SessionUsage[];
  periodDays: number;
};

type View = 'projects' | 'sessions';

// One card's worth of data: exact numbers from session logs when available,
// otherwise the old activity-weighted estimate (e.g. Antigravity, which logs
// no working directory).
type ProjectCardData = {
  id: string;
  label: string;
  path?: string;
  providers: ProviderId[];
  lastSeen?: string;
  costUsd: number;
  todayCostUsd: number;
  tokens: number;
  models: string[];
  sessions?: number;
  cacheHitRatio?: number;
  commands: ProjectCommandEntry[];
  estimated: boolean;
};

const formatAge = (timestamp?: string) => {
  if (!timestamp) return '';
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

const formatDuration = (seconds: number) => {
  if (seconds < 60) return '<1m';
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  return h > 0 ? `${h}h ${m}m` : `${m}m`;
};

const unixToIso = (ts: number | null) => (ts != null ? new Date(ts * 1000).toISOString() : undefined);

const shortModel = (model: string) => model.replace(/^claude-/, '');

const normalizePath = (path?: string | null) =>
  (path ?? '').trim().toLowerCase().replace(/\\/g, '/').replace(/\/+$/, '');

const basename = (path: string) => path.replace(/\\/g, '/').replace(/\/+$/, '').split('/').filter(Boolean).pop() ?? path;

const cacheHit = (s: Pick<SessionUsage, 'input_tokens' | 'cache_read_tokens' | 'cache_write_tokens'>) => {
  const denominator = s.input_tokens + s.cache_read_tokens + s.cache_write_tokens;
  return denominator > 0 ? s.cache_read_tokens / denominator : 0;
};

const commandLabel = (prompt: string) => {
  const compact = prompt.replace(/\s+/g, ' ').trim();
  return compact.length > 72 ? `${compact.slice(0, 69)}...` : compact;
};

const providerColors: Record<ProviderId, string> = {
  claude: '#d97757',
  codex: '#10a37f',
  antigravity: '#4f6bed',
};

const providerAccents: Record<ProviderId, string> = {
  claude: '217 119 87',
  codex: '16 163 127',
  antigravity: '79 107 237',
};

// Exact projects first (they carry real cost), each enriched with the recent
// commands the activity heuristic found under the same path; then any
// heuristic-only projects, flagged as estimates.
const mergeProjects = (exact: ProjectUsage[], heuristic: ProjectSummary[]): ProjectCardData[] => {
  const byPath = new Map<string, ProjectSummary>();
  heuristic.forEach((summary) => {
    if (summary.path) byPath.set(normalizePath(summary.path), summary);
  });
  const used = new Set<string>();

  const cards: ProjectCardData[] = exact.map((project) => {
    const match = project.path ? byPath.get(normalizePath(project.path)) : undefined;
    if (match) used.add(match.id);
    return {
      id: project.project_id,
      label: project.label,
      path: project.path ?? undefined,
      providers: project.providers,
      lastSeen: unixToIso(project.last_ts),
      costUsd: project.cost_usd,
      todayCostUsd: project.today_cost_usd,
      tokens: project.total_tokens,
      models: project.models.map((m) => m.model),
      sessions: project.session_count,
      cacheHitRatio: project.cache_hit_ratio,
      commands: match?.commands ?? [],
      estimated: false,
    };
  });

  const exactProviders = new Set(exact.flatMap((p) => p.providers));
  heuristic
    .filter((summary) => !used.has(summary.id))
    // Claude/Codex activity without an exact match is already counted under
    // another path (or outside the period); only keep providers we can't attribute.
    .filter((summary) => summary.providers.some((p) => p === 'antigravity' || !exactProviders.has(p)))
    .forEach((summary) =>
      cards.push({
        id: summary.id,
        label: summary.label,
        path: summary.path,
        providers: summary.providers,
        lastSeen: summary.latest_timestamp,
        costUsd: summary.estimated_cost_usd,
        todayCostUsd: summary.estimated_cost_today_usd,
        tokens: summary.estimated_tokens,
        models: summary.models,
        commands: summary.commands,
        estimated: true,
      })
    );
  return cards;
};

const ProjectCard = ({ project }: { project: ProjectCardData }) => {
  const primaryProvider = project.providers[0];

  return (
    <div
      className="proj-card"
      style={{ '--widget-accent': providerAccents[primaryProvider] ?? '148 163 184' } as React.CSSProperties}
    >
      {/* Header: title + cost */}
      <div className="proj-card-header">
        <div className="proj-card-title-col">
          <div className="proj-card-title-row">
            <span className="proj-card-title">{project.label}</span>
            <span className="proj-card-age">{formatAge(project.lastSeen)}</span>
          </div>
          <span className="proj-card-path" title={project.path ?? project.label}>
            {project.path ?? 'Terminal/session activity'}
          </span>
        </div>
        <div className="proj-card-cost-col">
          <span className="proj-card-cost">${project.costUsd.toFixed(2)}</span>
          <span className="proj-card-cost-sub">today ${project.todayCostUsd.toFixed(2)}</span>
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
        {project.estimated && (
          <span className="proj-chip proj-chip-estimated" title="Cost split from daily totals by activity — this provider logs no project path">
            est.
          </span>
        )}
        {project.sessions != null && (
          <span className="proj-chip">{project.sessions} {project.sessions === 1 ? 'session' : 'sessions'}</span>
        )}
        <span className="proj-chip">{formatTokens(project.tokens)} tok</span>
        {project.cacheHitRatio != null && project.tokens > 0 && (
          <span className="proj-chip" title="Share of input tokens served from the prompt cache">
            cache {Math.round(project.cacheHitRatio * 100)}%
          </span>
        )}
        {project.models.slice(0, 2).map((model) => (
          <span key={`${project.id}-${model}`} className="proj-chip proj-chip-model" title={model}>
            {shortModel(model)}
          </span>
        ))}
        {project.models.length > 2 && (
          <span className="proj-chip">+{project.models.length - 2}</span>
        )}
      </div>

      {/* Command timeline */}
      {project.commands.length > 0 && (
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
      )}
    </div>
  );
};

const SessionRow = ({ session }: { session: SessionUsage }) => {
  const label = session.cwd ? basename(session.cwd) : 'Unattributed';
  const duration =
    session.first_ts != null && session.last_ts != null ? formatDuration(session.last_ts - session.first_ts) : null;
  const hit = cacheHit(session);

  return (
    <div
      className="proj-session"
      style={{ '--widget-accent': providerAccents[session.provider] } as React.CSSProperties}
    >
      <span className="proj-chip proj-chip-provider" style={{ borderColor: providerColors[session.provider], color: providerColors[session.provider] }}>
        <ProviderLogo provider={session.provider} size={10} />
      </span>
      <div className="proj-session-main">
        <div className="proj-session-title-row">
          <span className="proj-session-title" title={session.cwd ?? session.session_id}>{label}</span>
          <span className="proj-card-age">{formatAge(unixToIso(session.last_ts))}</span>
        </div>
        <div className="proj-card-chips">
          {duration && <span className="proj-chip" title="First to last logged message">{duration}</span>}
          <span className="proj-chip">{formatTokens(session.total_tokens)} tok</span>
          {session.total_tokens > 0 && <span className="proj-chip">cache {Math.round(hit * 100)}%</span>}
          {session.models.slice(0, 2).map((m) => (
            <span key={m.model} className="proj-chip proj-chip-model" title={`${m.model} · $${m.cost_usd.toFixed(2)}`}>
              {shortModel(m.model)}
            </span>
          ))}
        </div>
      </div>
      <div className="proj-card-cost-col">
        <span className="proj-card-cost">${session.cost_usd.toFixed(2)}</span>
        {session.today_cost_usd > 0 && <span className="proj-card-cost-sub">today ${session.today_cost_usd.toFixed(2)}</span>}
      </div>
    </div>
  );
};

const ProjectOverview = ({ recentActivity, costHistory, projectUsage, sessionUsage, periodDays }: ProjectOverviewProps) => {
  const [view, setView] = useState<View>('projects');

  const projects = useMemo(
    () =>
      mergeProjects(
        projectUsage,
        buildProjectSummaries(recentActivity, costHistory, {
          maxProjects: 24,
          maxCommandsPerProject: 3,
        })
      ),
    [costHistory, recentActivity, projectUsage]
  );

  const exactTotal = projectUsage.reduce((sum, p) => sum + p.cost_usd, 0);

  return (
    <div className="proj-root">
      {/* Header bar */}
      <div className="proj-header">
        <div className="proj-header-left">
          <span className="proj-header-title">{view === 'projects' ? 'Projects' : 'Sessions'}</span>
          <span className="proj-header-subtitle" title={
              periodDays > 30
                ? 'Exact cost from the Claude and Codex session logs still on disk. Claude Code deletes logs after 30 days by default, so older spend is not attributed to projects.'
                : 'Exact cost from local Claude and Codex session logs'
            }>
            {periodDays}d · ${exactTotal.toFixed(2)}
            {periodDays > 30 && ' · logs on disk only'}
          </span>
        </div>
        <div className="proj-header-right">
          <div className="period-selector-group proj-view-toggle">
            <GlassPill active={view === 'projects'} onClick={() => setView('projects')}>Projects</GlassPill>
            <GlassPill active={view === 'sessions'} onClick={() => setView('sessions')}>Sessions</GlassPill>
          </div>
          <span className="proj-header-badge">
            {view === 'projects' ? `${projects.length} projects` : `${sessionUsage.length} sessions`}
          </span>
        </div>
      </div>

      {view === 'projects' ? (
        projects.length === 0 ? (
          <div className="proj-empty">
            No project activity detected yet. Use a provider CLI or workspace to generate activity.
          </div>
        ) : (
          <div className="proj-grid soft-scroll">
            {projects.map((project) => (
              <ProjectCard key={project.id} project={project} />
            ))}
          </div>
        )
      ) : sessionUsage.length === 0 ? (
        <div className="proj-empty">
          No Claude or Codex sessions in the last {periodDays} days.
        </div>
      ) : (
        <div className="proj-session-list soft-scroll">
          {sessionUsage.map((session) => (
            <SessionRow key={`${session.provider}-${session.session_id}`} session={session} />
          ))}
        </div>
      )}
    </div>
  );
};

export default ProjectOverview;
