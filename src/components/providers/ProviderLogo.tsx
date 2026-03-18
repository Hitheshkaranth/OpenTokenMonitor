import { ProviderId } from '@/types';

type ProviderLogoProps = {
  provider: ProviderId;
  size?: number;
};

const srcByProvider: Record<ProviderId, string> = {
  claude: '/providers/claude.png',
  codex: '/providers/openai.png',
  gemini: '/providers/geminai.webp',
};

const ProviderLogo = ({ provider, size = 18 }: ProviderLogoProps) => (
  <img
    src={srcByProvider[provider]}
    alt=""
    aria-hidden="true"
    width={size}
    height={size}
    style={{
      width: size,
      height: size,
      borderRadius: '50%',
      objectFit: 'cover',
      background: 'rgba(255, 255, 255, 0.9)',
      padding: Math.max(1, size * 0.08),
      boxShadow: '0 0 0 1px rgba(255,255,255,0.15)',
      flexShrink: 0,
    }}
  />
);

export default ProviderLogo;
