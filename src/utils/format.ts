const trimZero = (value: string) => value.replace(/\.0$/, '');

// Compact dollars so large totals fit the small cards: $162.73, $4K, $4.1K, $1.2M.
export const formatUsd = (value: number) => {
  const v = Number.isFinite(value) ? value : 0;
  const abs = Math.abs(v);
  if (abs >= 1_000_000) return `$${trimZero((v / 1_000_000).toFixed(1))}M`;
  if (abs >= 1_000) return `$${trimZero((v / 1_000).toFixed(1))}K`;
  return `$${v.toFixed(2)}`;
};
