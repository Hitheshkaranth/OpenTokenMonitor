import { PropsWithChildren, useState } from 'react';

type GlassTooltipProps = PropsWithChildren<{ text: string; position?: 'top' | 'bottom' | 'left' | 'right' }>;

const GlassTooltip = ({ text, children, position = 'top' }: GlassTooltipProps) => {
  const [visible, setVisible] = useState(false);
  const baseStyle =
    position === 'bottom'
      ? { top: 'calc(100% + 8px)', left: '50%', transform: 'translateX(-50%)' }
      : position === 'left'
        ? { right: 'calc(100% + 8px)', top: '50%', transform: 'translateY(-50%)' }
        : position === 'right'
          ? { left: 'calc(100% + 8px)', top: '50%', transform: 'translateY(-50%)' }
          : { bottom: 'calc(100% + 8px)', left: '50%', transform: 'translateX(-50%)' };

  return (
    <span
      style={{ position: 'relative', display: 'inline-flex' }}
      onMouseEnter={() => setVisible(true)}
      onMouseLeave={() => setVisible(false)}
    >
      {children}
      <span
        role="tooltip"
        className="glass-panel"
        style={{
          position: 'absolute',
          ...baseStyle,
          padding: '6px 10px',
          fontSize: 11,
          whiteSpace: 'nowrap',
          pointerEvents: 'none',
          opacity: visible ? 1 : 0,
          transition: 'opacity 0.2s ease',
          zIndex: 30,
        }}
      >
        {text}
      </span>
    </span>
  );
};

export default GlassTooltip;
