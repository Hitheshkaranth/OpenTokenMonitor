type EmptyStateProps = {
  onOpenSettings?: () => void;
  title?: string;
  message?: string;
  ctaLabel?: string;
};

const EmptyState = ({
  onOpenSettings,
  title = 'No providers configured',
  message = 'Enable at least one provider in settings to start tracking usage.',
  ctaLabel = 'Open Settings',
}: EmptyStateProps) => (
  <div className="glass-panel" style={{ padding: 16, display: 'grid', gap: 8, textAlign: 'center' }}>
    <div className="provider-name">{title}</div>
    <div className="metric-label">{message}</div>
    {onOpenSettings && (
      <button className="glass-pill" onClick={onOpenSettings} type="button">
        {ctaLabel}
      </button>
    )}
  </div>
);

export default EmptyState;
