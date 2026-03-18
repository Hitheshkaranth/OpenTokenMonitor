import { ChangeEvent, InputHTMLAttributes, ReactNode, useMemo, useState } from 'react';

type GlassInputProps = Omit<InputHTMLAttributes<HTMLInputElement>, 'onChange'> & {
  value: string;
  onChange: (value: string) => void;
  icon?: ReactNode;
  monospace?: boolean;
};

const GlassInput = ({
  type = 'text',
  value,
  onChange,
  placeholder,
  icon,
  monospace = false,
  style,
  ...props
}: GlassInputProps) => {
  const [revealed, setRevealed] = useState(false);
  const resolvedType = useMemo(() => {
    if (type !== 'password') return type;
    return revealed ? 'text' : 'password';
  }, [revealed, type]);

  const handleChange = (event: ChangeEvent<HTMLInputElement>) => onChange(event.target.value);

  return (
    <div className="glass-pill" style={{ display: 'grid', gridTemplateColumns: icon ? 'auto 1fr auto' : '1fr auto', gap: 8, ...style }}>
      {icon && <span style={{ display: 'inline-flex', alignItems: 'center' }}>{icon}</span>}
      <input
        {...props}
        type={resolvedType}
        value={value}
        onChange={handleChange}
        placeholder={placeholder}
        style={{
          background: 'transparent',
          border: 'none',
          outline: 'none',
          color: 'var(--text-primary)',
          fontFamily: monospace ? 'var(--font-mono)' : 'var(--font-ui)',
          minWidth: 0,
        }}
      />
      {type === 'password' && (
        <button
          type="button"
          className="glass-pill"
          onClick={() => setRevealed((v) => !v)}
          style={{ padding: '2px 6px', minHeight: 18 }}
          aria-label={revealed ? 'Hide value' : 'Show value'}
        >
          {revealed ? 'Hide' : 'Show'}
        </button>
      )}
    </div>
  );
};

export default GlassInput;
