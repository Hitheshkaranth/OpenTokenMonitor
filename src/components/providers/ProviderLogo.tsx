import { ProviderId } from '@/types';

type ProviderLogoProps = {
  provider: ProviderId;
  size?: number;
};

const ProviderLogo = ({ provider, size = 14 }: ProviderLogoProps) => {
  const srcByProvider: Record<ProviderId, string> = {
    claude: '/providers/claude.png',
    codex: '/providers/openai.png',
    gemini: '/providers/geminai.webp',
  };

  return (
    <img
      src={srcByProvider[provider]}
      alt=""
      aria-hidden="true"
      width={size}
      height={size}
      style={{ width: size, height: size, borderRadius: '50%', objectFit: 'cover' }}
    />
  );
};

export default ProviderLogo;
