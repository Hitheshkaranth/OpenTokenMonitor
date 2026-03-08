type GlassToggleProps = {
  checked: boolean;
  onChange: (next: boolean) => void;
  label: string;
};

const GlassToggle = ({ checked, onChange, label }: GlassToggleProps) => {
  return (
    <button
      type="button"
      className="glass-pill"
      onClick={() => onChange(!checked)}
      style={{
        minWidth: 92,
        justifyContent: 'space-between',
        cursor: 'pointer',
      }}
      aria-pressed={checked}
      aria-label={label}
    >
      <span style={{ fontSize: 12 }}>{label}</span>
      <span
        style={{
          width: 24,
          height: 14,
          borderRadius: 99,
          border: '1px solid var(--glass-border)',
          background: checked ? 'var(--toggle-track-on)' : 'var(--toggle-track-off)',
          position: 'relative',
        }}
      >
        <span
          style={{
            position: 'absolute',
            top: 1,
            left: checked ? 11 : 1,
            width: 10,
            height: 10,
            borderRadius: 99,
            background: 'var(--toggle-knob)',
            transition: 'left .2s ease',
          }}
        />
      </span>
    </button>
  );
};

export default GlassToggle;
