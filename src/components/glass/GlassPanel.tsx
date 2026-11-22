import { CSSProperties, KeyboardEvent, PropsWithChildren, useRef } from 'react';
import { useSpecularHighlight } from '@/hooks/useSpecularHighlight';

type Tint = 'claude' | 'codex' | 'antigravity' | 'neutral';

type GlassPanelProps = PropsWithChildren<{
  className?: string;
  tint?: Tint;
  style?: CSSProperties;
  onClick?: () => void;
  title?: string;
  ariaLabel?: string;
}>;

const tintClass: Record<Tint, string> = {
  claude: 'glass-claude',
  codex: 'glass-codex',
  antigravity: 'glass-antigravity',
  neutral: '',
};

// Enter/Space activation for div-based clickable surfaces.
export const activateOnKey = (onClick?: () => void) => (event: KeyboardEvent) => {
  if (!onClick || event.target !== event.currentTarget) return;
  if (event.key === 'Enter' || event.key === ' ') {
    event.preventDefault();
    onClick();
  }
};

const GlassPanel = ({ children, className = '', tint = 'neutral', style, onClick, title, ariaLabel }: GlassPanelProps) => {
  const ref = useRef<HTMLDivElement>(null);
  useSpecularHighlight(ref);
  return (
    <div
      ref={ref}
      className={`glass-panel ${tintClass[tint]} ${className}`.trim()}
      style={style}
      onClick={onClick}
      title={title}
      role={onClick ? 'button' : undefined}
      tabIndex={onClick ? 0 : undefined}
      aria-label={onClick ? ariaLabel : undefined}
      onKeyDown={onClick ? activateOnKey(onClick) : undefined}
    >
      {children}
    </div>
  );
};

export default GlassPanel;
