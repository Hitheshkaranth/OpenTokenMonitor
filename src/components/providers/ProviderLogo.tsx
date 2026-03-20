import { ProviderId } from '@/types';

type ProviderLogoProps = {
  provider: ProviderId;
  size?: number;
  variant?: 'default' | 'widget-core';
};

const srcByProvider: Record<ProviderId, string> = {
  claude: '/providers/claude-ai-icon.png',
  codex: '/providers/chatgpt-icon.png',
  gemini: '/providers/google-gemini-icon.png',
};

const widgetCoreOptics: Record<ProviderId, { scale: number; x: number; y: number }> = {
  claude: { scale: 0.9, x: 0, y: -0.2 },
  codex: { scale: 0.92, x: 0, y: 0 },
  gemini: { scale: 1.08, x: 0, y: -0.1 },
};

const ProviderLogo = ({ provider, size = 18, variant = 'default' }: ProviderLogoProps) => {
  const optics = variant === 'widget-core'
    ? widgetCoreOptics[provider]
    : { scale: 1, x: 0, y: 0 };

  return (
    <img
      src={srcByProvider[provider]}
      alt=""
      aria-hidden="true"
      width={size}
      height={size}
      style={{
        width: size,
        height: size,
        display: 'block',
        borderRadius: variant === 'widget-core' ? 0 : '50%',
        objectFit: 'contain',
        background: variant === 'widget-core' ? 'transparent' : 'rgba(255, 255, 255, 0.9)',
        padding: variant === 'widget-core' ? 0 : Math.max(1, size * 0.08),
        boxShadow: variant === 'widget-core' ? 'none' : '0 0 0 1px rgba(255,255,255,0.15)',
        transform: `translate(${optics.x}px, ${optics.y}px) scale(${optics.scale})`,
        transformOrigin: '50% 50%',
        filter: variant === 'widget-core' ? 'drop-shadow(0 0 1px rgba(255,255,255,0.24))' : 'none',
        flexShrink: 0,
      }}
    />
  );
};

export default ProviderLogo;
