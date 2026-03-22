import {
  CostEntry,
  ProjectCommandEntry,
  ProjectSummary,
  ProviderId,
  RecentActivityEntry,
} from '@/types';

type ProviderRecord<T> = Record<ProviderId, T>;

const sanitizeKey = (value: string) =>
  value
    .trim()
    .toLowerCase()
    .replace(/\\/g, '/')
    .replace(/\/+/g, '/');

const basename = (value: string) => {
  const normalized = value.replace(/\\/g, '/').replace(/\/+$/, '');
  const parts = normalized.split('/').filter(Boolean);
  return parts[parts.length - 1] ?? value;
};

const localIsoDay = (date: Date) => {
  const year = date.getFullYear();
  const month = `${date.getMonth() + 1}`.padStart(2, '0');
  const day = `${date.getDate()}`.padStart(2, '0');
  return `${year}-${month}-${day}`;
};

const projectFromEntry = (entry: RecentActivityEntry) => {
  if (entry.cwd?.trim()) {
    const path = entry.cwd.trim();
    return {
      id: `cwd:${sanitizeKey(path)}`,
      label: basename(path),
      path,
    };
  }

  if (entry.terminal_label?.trim()) {
    const label = entry.terminal_label.trim();
    return {
      id: `terminal:${sanitizeKey(label)}`,
      label,
      path: undefined,
    };
  }

  if (entry.session_id?.trim()) {
    const sessionId = entry.session_id.trim();
    return {
      id: `session:${entry.provider}:${sanitizeKey(sessionId)}`,
      label: `${entry.provider} session`,
      path: undefined,
    };
  }

  return {
    id: `provider:${entry.provider}:unscoped`,
    label: `${entry.provider} workspace`,
    path: undefined,
  };
};

const isoDay = (value: string) => {
  const date = new Date(value);
  if (!Number.isFinite(date.getTime())) return '';
  return localIsoDay(date);
};

const todayIsoDay = () => localIsoDay(new Date());

const normalizedModel = (value?: string) => {
  const trimmed = value?.trim();
  return trimmed && trimmed.length > 0 ? trimmed : '*';
};

const providerModelKey = (provider: ProviderId, date: string, model: string) =>
  `${provider}|${date}|${sanitizeKey(model)}`;

const providerDayKey = (provider: ProviderId, date: string) => `${provider}|${date}`;

export const buildProjectSummaries = (
  recentActivity: ProviderRecord<RecentActivityEntry[]>,
  costHistory: ProviderRecord<CostEntry[]>,
  options?: { maxProjects?: number; maxCommandsPerProject?: number }
): ProjectSummary[] => {
  const maxProjects = options?.maxProjects ?? 8;
  const maxCommandsPerProject = options?.maxCommandsPerProject ?? 5;
  const flattened = (Object.entries(recentActivity) as [ProviderId, RecentActivityEntry[]][])
    .flatMap(([provider, entries]) =>
      entries.map((entry) => {
        const project = projectFromEntry(entry);
        return {
          ...entry,
          provider,
          project_id: project.id,
          project_label: project.label,
          project_path: project.path,
        } satisfies ProjectCommandEntry;
      })
    )
    .filter((entry) => entry.prompt.trim().length > 0)
    .sort((a, b) => new Date(b.timestamp).getTime() - new Date(a.timestamp).getTime());

  const projects = new Map<string, ProjectSummary>();
  const modelCounts = new Map<string, Map<string, number>>();
  const dayCounts = new Map<string, Map<string, number>>();

  for (const entry of flattened) {
    const modelKey = providerModelKey(entry.provider, isoDay(entry.timestamp), normalizedModel(entry.model));
    const modelMap = modelCounts.get(modelKey) ?? new Map<string, number>();
    modelMap.set(entry.project_id, (modelMap.get(entry.project_id) ?? 0) + 1);
    modelCounts.set(modelKey, modelMap);

    const dayKey = providerDayKey(entry.provider, isoDay(entry.timestamp));
    const dayMap = dayCounts.get(dayKey) ?? new Map<string, number>();
    dayMap.set(entry.project_id, (dayMap.get(entry.project_id) ?? 0) + 1);
    dayCounts.set(dayKey, dayMap);

    const existing = projects.get(entry.project_id);
    if (!existing) {
      projects.set(entry.project_id, {
        id: entry.project_id,
        label: entry.project_label,
        path: entry.project_path,
        latest_timestamp: entry.timestamp,
        activity_count: 1,
        providers: [entry.provider],
        models: entry.model ? [entry.model] : [],
        estimated_cost_usd: 0,
        estimated_cost_today_usd: 0,
        estimated_tokens: 0,
        commands: [entry],
      });
      continue;
    }

    existing.activity_count += 1;
    if (new Date(entry.timestamp).getTime() > new Date(existing.latest_timestamp).getTime()) {
      existing.latest_timestamp = entry.timestamp;
    }
    if (!existing.providers.includes(entry.provider)) {
      existing.providers.push(entry.provider);
    }
    if (entry.model && !existing.models.includes(entry.model)) {
      existing.models.push(entry.model);
    }
    if (!existing.path && entry.project_path) {
      existing.path = entry.project_path;
    }
    existing.commands.push(entry);
  }

  const today = todayIsoDay();
  const costRows = (Object.values(costHistory) as CostEntry[][]).flat();

  for (const row of costRows) {
    const rowTokens =
      row.input_tokens + row.output_tokens + row.cache_read_tokens + row.cache_write_tokens;
    const exactKey = providerModelKey(row.provider, row.date, normalizedModel(row.model));
    const exactCounts = modelCounts.get(exactKey);
    const fallbackCounts = dayCounts.get(providerDayKey(row.provider, row.date));
    const counts = exactCounts && exactCounts.size > 0 ? exactCounts : fallbackCounts;
    if (!counts || counts.size === 0) {
      continue;
    }

    const totalWeight = Array.from(counts.values()).reduce((sum, value) => sum + value, 0);
    if (totalWeight <= 0) {
      continue;
    }

    for (const [projectId, weight] of counts.entries()) {
      const project = projects.get(projectId);
      if (!project) continue;
      const allocation = row.estimated_cost_usd * (weight / totalWeight);
      const tokenAllocation = rowTokens * (weight / totalWeight);
      project.estimated_cost_usd += allocation;
      project.estimated_tokens += tokenAllocation;
      if (row.date === today) {
        project.estimated_cost_today_usd += allocation;
      }
    }
  }

  return Array.from(projects.values())
    .map((project) => ({
      ...project,
      providers: [...project.providers].sort(),
      models: [...project.models].sort(),
      estimated_tokens: Math.round(project.estimated_tokens),
      commands: [...project.commands]
        .sort((a, b) => new Date(a.timestamp).getTime() - new Date(b.timestamp).getTime())
        .slice(-maxCommandsPerProject),
    }))
    .sort((a, b) => {
      if (b.estimated_cost_usd !== a.estimated_cost_usd) {
        return b.estimated_cost_usd - a.estimated_cost_usd;
      }
      return new Date(b.latest_timestamp).getTime() - new Date(a.latest_timestamp).getTime();
    })
    .slice(0, maxProjects);
};
