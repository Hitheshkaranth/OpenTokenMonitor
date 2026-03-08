import { PropsWithChildren } from 'react';

type GlassPillProps = PropsWithChildren<{
  onClick?: () => void;
  active?: boolean;
  title?: string;
  className?: string;
}>;

const GlassPill = ({ children, onClick, active = false, title, className = '' }: GlassPillProps) => {
  return (
    <button
      type="button"
      title={title}
      className={`glass-pill transition-smooth ${className}`.trim()}
      style={{
        cursor: 'pointer',
        opacity: active ? 1 : 0.82,
        borderColor: active ? 'var(--control-border-strong)' : undefined,
      }}
      onClick={onClick}
    >
      {children}
    </button>
  );
};

export default GlassPill;
