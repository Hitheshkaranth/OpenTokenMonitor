import { PropsWithChildren } from 'react';

type GlassTooltipProps = PropsWithChildren<{ text: string }>;

const GlassTooltip = ({ text, children }: GlassTooltipProps) => {
  return (
    <span style={{ position: 'relative', display: 'inline-flex' }}>
      {children}
      <span
        role="tooltip"
        className="glass-panel"
        style={{
          position: 'absolute',
          bottom: 'calc(100% + 8px)',
          left: '50%',
          transform: 'translateX(-50%)',
          padding: '6px 10px',
          fontSize: 11,
          whiteSpace: 'nowrap',
          pointerEvents: 'none',
          opacity: 0,
        }}
      >
        {text}
      </span>
    </span>
  );
};

export default GlassTooltip;
