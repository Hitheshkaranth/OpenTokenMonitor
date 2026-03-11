import { ButtonHTMLAttributes, PropsWithChildren } from 'react';

type Variant = 'default' | 'primary' | 'danger';
type Size = 'sm' | 'md';

type GlassButtonProps = PropsWithChildren<
  Omit<ButtonHTMLAttributes<HTMLButtonElement>, 'color'> & {
    variant?: Variant;
    size?: Size;
    providerColor?: string;
  }
>;

const sizeStyle: Record<Size, { padding: string; fontSize: number }> = {
  sm: { padding: '6px 10px', fontSize: 12 },
  md: { padding: '8px 12px', fontSize: 13 },
};

const GlassButton = ({
  children,
  variant = 'default',
  size = 'md',
  providerColor,
  disabled,
  className = '',
  style,
  ...props
}: GlassButtonProps) => {
  const chosenColor =
    providerColor ?? (variant === 'danger' ? '#f87171' : variant === 'primary' ? '#4f9eff' : undefined);
  const isColored = variant !== 'default' || !!providerColor;

  return (
    <button
      type="button"
      className={`glass-pill transition-smooth ${className}`.trim()}
      disabled={disabled}
      style={{
        ...sizeStyle[size],
        cursor: disabled ? 'not-allowed' : 'pointer',
        opacity: disabled ? 0.4 : 1,
        pointerEvents: disabled ? 'none' : undefined,
        background: isColored
          ? `linear-gradient(135deg, ${chosenColor}dd, ${chosenColor}99)`
          : undefined,
        boxShadow: isColored ? `0 0 0 1px ${chosenColor}55, 0 8px 20px ${chosenColor}33` : undefined,
        ...style,
      }}
      {...props}
    >
      {children}
    </button>
  );
};

export default GlassButton;

