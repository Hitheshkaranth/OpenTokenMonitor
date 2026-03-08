import { Area, AreaChart, ResponsiveContainer, Tooltip, XAxis, YAxis } from 'recharts';
import { TrendPoint } from '@/types';

type CostTrendChartProps = {
  points: TrendPoint[];
  color: string;
};

const CostTrendChart = ({ points, color }: CostTrendChartProps) => {
  return (
    <div className="glass-panel" style={{ height: 150, padding: 8 }}>
      <ResponsiveContainer width="100%" height="100%">
        <AreaChart data={points} margin={{ top: 8, right: 8, left: -16, bottom: 0 }}>
          <defs>
            <linearGradient id="costFill" x1="0" y1="0" x2="0" y2="1">
              <stop offset="0%" stopColor={color} stopOpacity={0.5} />
              <stop offset="100%" stopColor={color} stopOpacity={0.02} />
            </linearGradient>
          </defs>
          <XAxis dataKey="date" tick={{ fill: 'var(--text-muted)', fontSize: 10 }} />
          <YAxis tick={{ fill: 'var(--text-muted)', fontSize: 10 }} width={42} />
          <Tooltip
            contentStyle={{
              background: 'var(--tooltip-bg)',
              border: '1px solid var(--tooltip-border)',
              color: 'var(--text-primary)',
              borderRadius: 10,
            }}
          />
          <Area type="monotone" dataKey="cost_usd" stroke={color} fill="url(#costFill)" strokeWidth={2} />
        </AreaChart>
      </ResponsiveContainer>
    </div>
  );
};

export default CostTrendChart;
