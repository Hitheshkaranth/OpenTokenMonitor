const LoadingState = () => (
  <div className="glass-panel animate-shimmer" style={{ padding: 16, display: 'grid', gap: 10 }}>
    <div className="glass-pill" style={{ height: 18, width: '40%' }} />
    <div className="glass-pill" style={{ height: 12, width: '80%' }} />
    <div className="glass-pill" style={{ height: 72, width: '100%' }} />
    <div className="glass-pill" style={{ height: 12, width: '55%' }} />
  </div>
);

export default LoadingState;

