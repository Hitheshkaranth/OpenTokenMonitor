import { useEffect, useMemo, useState } from 'react';

export const useGlassTheme = (themeOverride?: 'light' | 'dark' | 'system') => {
  const [systemDark, setSystemDark] = useState<boolean>(() =>
    window.matchMedia('(prefers-color-scheme: dark)').matches
  );
  const [reducedMotion, setReducedMotion] = useState<boolean>(() =>
    window.matchMedia('(prefers-reduced-motion: reduce)').matches
  );

  useEffect(() => {
    const darkMq = window.matchMedia('(prefers-color-scheme: dark)');
    const motionMq = window.matchMedia('(prefers-reduced-motion: reduce)');
    const onDark = () => setSystemDark(darkMq.matches);
    const onMotion = () => setReducedMotion(motionMq.matches);
    darkMq.addEventListener('change', onDark);
    motionMq.addEventListener('change', onMotion);
    return () => {
      darkMq.removeEventListener('change', onDark);
      motionMq.removeEventListener('change', onMotion);
    };
  }, []);

  const resolved = useMemo<'light' | 'dark'>(() => {
    if (!themeOverride || themeOverride === 'system') return systemDark ? 'dark' : 'light';
    return themeOverride;
  }, [systemDark, themeOverride]);

  useEffect(() => {
    document.documentElement.setAttribute('data-theme', resolved);
  }, [resolved]);

  return {
    theme: resolved,
    reducedMotion,
  };
};
