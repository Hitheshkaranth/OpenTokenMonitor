import { CSSProperties, PropsWithChildren, useRef } from 'react';
import { useSpecularHighlight } from '@/hooks/useSpecularHighlight';

type Tint = 'claude' | 'codex' | 'gemini' | 'neutral';

type GlassPanelProps = PropsWithChildren<{
  className?: string;
  tint?: Tint;
  style?: CSSProperties;
}>;

const tintClass: Record<Tint, string> = {
  claude: 'glass-claude',
  codex: 'glass-codex',
  gemini: 'glass-gemini',
  neutral: '',
};

const GlassPanel = ({ children, className = '', tint = 'neutral', style }: GlassPanelProps) => {
  const ref = useRef<HTMLDivElement>(null);
  useSpecularHighlight(ref);
  return (
    <div ref={ref} className={`glass-panel ${tintClass[tint]} ${className}`.trim()} style={style}>
      {children}
    </div>
  );
};

export default GlassPanel;
