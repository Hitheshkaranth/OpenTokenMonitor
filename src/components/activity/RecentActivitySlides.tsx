import { useEffect, useState } from 'react';
import { ChevronLeft, ChevronRight } from 'lucide-react';
import { RecentActivityEntry } from '@/types';

type RecentActivitySlidesProps = {
  entries: RecentActivityEntry[];
  emptyMessage: string;
  variant?: 'panel' | 'widget';
  resetKey?: string;
};

const formatAge = (timestamp: string) => {
  const deltaMs = Date.now() - new Date(timestamp).getTime();
  if (!Number.isFinite(deltaMs) || deltaMs < 0) return 'now';
  const minutes = Math.floor(deltaMs / 60_000);
  if (minutes < 1) return 'now';
  if (minutes < 60) return `${minutes}m`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h`;
  const days = Math.floor(hours / 24);
  return `${days}d`;
};

const recentModelLabel = (entry: RecentActivityEntry) => entry.model ?? 'model unavailable';

const fallbackTerminalLabel = (entry?: RecentActivityEntry) => {
  if (!entry) return 'local session';
  if (entry.terminal_label) return entry.terminal_label;
  if (entry.cwd) {
    const parts = entry.cwd.split(/[\\/]/).filter(Boolean);
    return parts[parts.length - 1] ?? entry.cwd;
  }
  return entry.session_id ? `session ${entry.session_id.slice(0, 6)}` : 'local session';
};

const RecentActivitySlides = ({
  entries,
  emptyMessage,
  variant = 'panel',
  resetKey,
}: RecentActivitySlidesProps) => {
  const [activeIndex, setActiveIndex] = useState(0);

  useEffect(() => {
    if (entries.length === 0) {
      setActiveIndex(0);
      return;
    }

    setActiveIndex((current) => Math.min(current, entries.length - 1));
  }, [entries.length]);

  useEffect(() => {
    setActiveIndex(0);
  }, [resetKey]);

  if (entries.length === 0) {
    return (
      <div className={`recent-activity-empty recent-activity-empty--${variant}`}>
        {emptyMessage}
      </div>
    );
  }

  const goPrev = () => {
    setActiveIndex((current) => (current === 0 ? entries.length - 1 : current - 1));
  };

  const goNext = () => {
    setActiveIndex((current) => (current === entries.length - 1 ? 0 : current + 1));
  };

  const iconSize = variant === 'widget' ? 12 : 14;

  return (
    <div className={`recent-activity-slides recent-activity-slides--${variant}`}>
      <div className={`recent-activity-stage recent-activity-stage--${variant}`}>
        <div
          className="recent-activity-track"
          style={{ transform: `translateX(-${activeIndex * 100}%)` }}
        >
          {entries.map((entry, index) => (
            <article
              key={`${entry.timestamp}-${entry.session_id ?? index}`}
              className={`recent-activity-card recent-activity-card--${variant}`}
              title={entry.cwd ?? entry.prompt}
            >
              <div className={`recent-activity-card-head recent-activity-card-head--${variant}`}>
                <span className={`recent-activity-terminal recent-activity-terminal--${variant}`}>
                  {fallbackTerminalLabel(entry)}
                </span>
                <span
                  className={`glass-pill recent-activity-model recent-activity-model--${variant}`}
                  title={entry.model ? `Model: ${entry.model}` : 'Model could not be detected for this conversation.'}
                >
                  {recentModelLabel(entry)}
                </span>
                <span className={`metric-label recent-activity-age recent-activity-age--${variant}`}>
                  {formatAge(entry.timestamp)}
                </span>
              </div>
              <div className={`recent-activity-prompt recent-activity-prompt--${variant}`}>
                {entry.prompt}
              </div>
            </article>
          ))}
        </div>
      </div>

      {entries.length > 1 && (
        <div className={`recent-activity-footer recent-activity-footer--${variant}`}>
          <div className={`recent-activity-nav recent-activity-nav--${variant}`}>
            <button
              type="button"
              className={`recent-activity-nav-btn recent-activity-nav-btn--${variant}`}
              onClick={goPrev}
              aria-label="Show previous recent conversation"
              title="Previous conversation"
            >
              <ChevronLeft size={iconSize} />
            </button>
            <span className={`glass-pill recent-activity-counter recent-activity-counter--${variant}`}>
              {activeIndex + 1} / {entries.length}
            </span>
            <button
              type="button"
              className={`recent-activity-nav-btn recent-activity-nav-btn--${variant}`}
              onClick={goNext}
              aria-label="Show next recent conversation"
              title="Next conversation"
            >
              <ChevronRight size={iconSize} />
            </button>
          </div>

          <div className={`recent-activity-dots recent-activity-dots--${variant}`}>
            {entries.map((entry, index) => (
              <button
                key={`${entry.timestamp}-${entry.session_id ?? index}-dot`}
                type="button"
                className={`recent-activity-dot ${index === activeIndex ? 'is-active' : ''}`}
                onClick={() => setActiveIndex(index)}
                aria-label={`Show recent conversation ${index + 1}`}
                title={`Conversation ${index + 1}`}
              />
            ))}
          </div>
        </div>
      )}
    </div>
  );
};

export default RecentActivitySlides;
