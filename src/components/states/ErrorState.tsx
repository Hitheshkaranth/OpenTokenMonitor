type ErrorStateProps = {
  message: string;
  onRetry?: () => void;
};

const ErrorState = ({ message, onRetry }: ErrorStateProps) => (
  <div className="glass-panel" style={{ padding: 16, display: 'grid', gap: 8 }}>
    <div className="provider-name" style={{ color: 'var(--status-critical)' }}>Something went wrong</div>
    <div className="metric-label">{message}</div>
    {onRetry && (
      <button className="glass-pill" type="button" onClick={onRetry} style={{ justifySelf: 'start' }}>
        Retry
      </button>
    )}
  </div>
);

export default ErrorState;

