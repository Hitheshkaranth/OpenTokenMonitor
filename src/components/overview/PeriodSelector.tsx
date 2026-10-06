import { GlassInput, GlassPill } from '@/components/glass';
import { TrendPreset } from '@/types';
import { useSettingsStore } from '@/stores/settingsStore';

type Option = { label: string; days: TrendPreset };

const presets: Option[] = [
  { label: '7d', days: 7 },
  { label: '30d', days: 30 },
  { label: '90d', days: 90 },
];

const PeriodSelector = () => {
  const trendPreset = useSettingsStore((s) => s.trendPreset);
  const trendCustom = useSettingsStore((s) => s.trendCustom);
  const customDays = useSettingsStore((s) => s.customDays);
  const setTrendPreset = useSettingsStore((s) => s.setTrendPreset);
  const setTrendCustom = useSettingsStore((s) => s.setTrendCustom);
  const setCustomDays = useSettingsStore((s) => s.setCustomDays);

  return (
    <div className="period-selector">
      <div className="period-selector-group">
        {presets.map((option) => {
          const active = !trendCustom && option.days === trendPreset;
          return (
            <GlassPill
              key={option.days}
              active={active}
              onClick={() => setTrendPreset(option.days as never)}
            >
              {option.label}
            </GlassPill>
          );
        })}
        <GlassPill active={trendCustom} onClick={() => { setTrendCustom(true); if (customDays <= 0 || customDays > 365) setCustomDays(14); }}>
          Custom
        </GlassPill>
      </div>

      {trendCustom ? (
        <div className="period-selector-custom">
          <GlassInput
            type="number"
            min={1}
            max={365}
            value={customDays > 0 ? String(customDays) : ''}
            onChange={(next) => setCustomDays(next === '' ? 0 : Number(next))}
            style={{ minWidth: 56 }}
          />
        </div>
      ) : null}
    </div>
  );
};

export default PeriodSelector;