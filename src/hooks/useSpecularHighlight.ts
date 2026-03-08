import { RefObject, useEffect } from 'react';

export const useSpecularHighlight = <T extends HTMLElement>(ref: RefObject<T | null>) => {
  useEffect(() => {
    const el = ref.current;
    if (!el) return;

    let raf = 0;
    const onMove = (event: MouseEvent) => {
      if (raf) return;
      raf = requestAnimationFrame(() => {
        raf = 0;
        const rect = el.getBoundingClientRect();
        el.style.setProperty('--sx', `${event.clientX - rect.left}px`);
        el.style.setProperty('--sy', `${event.clientY - rect.top}px`);
      });
    };

    el.addEventListener('mousemove', onMove);
    return () => {
      if (raf) cancelAnimationFrame(raf);
      el.removeEventListener('mousemove', onMove);
    };
  }, [ref]);
};
