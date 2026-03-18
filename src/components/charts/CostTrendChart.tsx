import { Area, AreaChart, ResponsiveContainer, Tooltip, XAxis, YAxis } from 'recharts';
import { TrendPoint } from '@/types';

type CostTrendChartProps = {
  points: TrendPoint[];
  color: string;
  compact?: boolean;
};

const CostTrendChart = ({ points, color, compact }: CostTrendChartProps) => {
  const height = compact ? 80 : 150;

  return (
    <div style={{ height, padding: compact ? 0 : 8 }}>
      <ResponsiveContainer width="100%" height="100%">
        <AreaChart data={points} margin={{ top: 4, right: 4, left: compact ? -20 : -16, bottom: 0 }}>
          <defs>
            <linearGradient id="costFill" x1="0" y1="0" x2="0" y2="1">
              <stop offset="0%" stopColor={color} stopOpacity={0.5} />
              <stop offset="100%" stopColor={color} stopOpacity={0.02} />
            </linearGradient>
          </defs>
          <XAxis
            dataKey="date"
            tick={{ fill: 'var(--text-muted)', fontSize: compact ? 8 : 10 }}
            tickLine={!compact}
            axisLine={!compact}
          />
          <YAxis
            tick={{ fill: 'var(--text-muted)', fontSize: compact ? 8 : 10 }}
            width={compact ? 30 : 42}
            tickLine={!compact}
            axisLine={!compact}
          />
          <Tooltip
            contentStyle={{
              background: 'var(--tooltip-bg)',
              border: '1px solid var(--tooltip-border)',
              color: 'var(--text-primary)',
              borderRadius: 10,
              fontSize: compact ? 10 : 12,
            }}
          />
          <Area type="monotone" dataKey="cost_usd" stroke={color} fill="url(#costFill)" strokeWidth={compact ? 1.5 : 2} />
        </AreaChart>
      </ResponsiveContainer>
    </div>
  );
};

export default CostTrendChart;
