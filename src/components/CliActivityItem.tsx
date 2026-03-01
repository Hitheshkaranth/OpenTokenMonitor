/**
 * CliActivityItem
 *
 * Renders a single entry from the CLI history feed.
 * Supports Claude Code (anthropic) and OpenAI Codex (openai) providers.
 * Each card shows the provider label, timestamp, command text, and optional project path.
 */
import React from 'react';
import { Clock } from 'lucide-react';
import { CliActivity } from '../types';

// Map provider IDs to human-readable labels and accent colours
const PROVIDER_META: Record<string, { label: string; color: string }> = {
  anthropic: { label: 'Claude Code', color: 'var(--claude-primary)' },
  openai:    { label: 'Codex',       color: 'var(--openai-primary)' },
};

const DEFAULT_META = { label: 'Unknown', color: 'var(--text-secondary)' };

interface Props {
  item: CliActivity;
}

const CliActivityItem: React.FC<Props> = ({ item }) => {
  const { label, color } = PROVIDER_META[item.provider] ?? DEFAULT_META;
  const time = new Date(item.timestamp).toLocaleTimeString();

  return (
    <div
      className="animate-slide-up"
      style={{
        background: 'rgba(255,255,255,0.03)',
        borderRadius: '10px',
        padding: '9px 10px',
        borderLeft: `3px solid ${color}`,
      }}
    >
      {/* Row 1: provider label + timestamp */}
      <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: '3px' }}>
        <span style={{
          fontSize: '11px', fontWeight: 700,
          textTransform: 'uppercase', letterSpacing: '0.04em',
          color: 'var(--text-secondary)',
        }}>
          {label}
        </span>
        <span style={{
          fontSize: '10px', color: 'var(--text-muted)',
          display: 'flex', alignItems: 'center', gap: '3px',
        }}>
          <Clock size={9} /> {time}
        </span>
      </div>

      {/* Row 2: command text in monospace */}
      <div style={{
        fontSize: '12px', fontFamily: 'var(--font-mono)',
        color: 'var(--text-primary)', wordBreak: 'break-all', lineHeight: 1.4,
      }}>
        <span style={{ color, marginRight: '5px' }}>&gt;</span>
        {item.command}
      </div>

      {/* Row 3: project path (optional) */}
      {item.project && (
        <div style={{
          fontSize: '10px', color: 'var(--text-muted)',
          marginTop: '3px', whiteSpace: 'nowrap',
          overflow: 'hidden', textOverflow: 'ellipsis',
        }}>
          {item.project}
        </div>
      )}
    </div>
  );
};

export default CliActivityItem;
