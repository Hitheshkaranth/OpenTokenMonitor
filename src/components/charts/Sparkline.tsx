import { Area, AreaChart, ResponsiveContainer } from 'recharts';
import { TrendPoint } from '@/types';

type SparklineProps = {
  points: TrendPoint[];
  color: string;
  height?: number;
};

const Sparkline = ({ points, color, height = 60 }: SparklineProps) => {
  if (points.length === 0) {
    return (
      <div style={{ height, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
        <span className="metric-label" style={{ fontSize: 10 }}>No trend data</span>
      </div>
    );
  }

  const gradientId = `spark-${color.replace('#', '')}`;

  return (
    <div style={{ height, width: '100%' }}>
      <ResponsiveContainer width="100%" height="100%">
        <AreaChart data={points} margin={{ top: 2, right: 2, left: 2, bottom: 2 }}>
          <defs>
            <linearGradient id={gradientId} x1="0" y1="0" x2="0" y2="1">
              <stop offset="0%" stopColor={color} stopOpacity={0.4} />
              <stop offset="100%" stopColor={color} stopOpacity={0.02} />
            </linearGradient>
          </defs>
          <Area
            type="monotone"
            dataKey="cost_usd"
            stroke={color}
            fill={`url(#${gradientId})`}
            strokeWidth={1.5}
            dot={false}
            isAnimationActive={false}
          />
        </AreaChart>
      </ResponsiveContainer>
    </div>
  );
};

export default Sparkline;
