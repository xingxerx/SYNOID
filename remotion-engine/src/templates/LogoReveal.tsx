import { AbsoluteFill, useCurrentFrame, useVideoConfig, interpolate, spring, Img } from 'remotion';
export interface LogoRevealProps { logoUrl: string; style?: 'fade-scale' | 'glitch' | 'particles' | 'morph'; backgroundColor?: string; }
export const LogoReveal: React.FC<LogoRevealProps> = ({ logoUrl, style = 'fade-scale', backgroundColor = '#0a0a0a' }) => {
    const frame = useCurrentFrame(); const { fps } = useVideoConfig();
    const scale = spring({ frame, fps, config: { damping: 12, stiffness: 100 } });
    const opacity = interpolate(frame, [0, 20], [0, 1], { extrapolateRight: 'clamp' });
    return (<AbsoluteFill style={{ backgroundColor, justifyContent: 'center', alignItems: 'center', display: 'flex' }}>
        <div style={{ transform: `scale(${scale})`, opacity }}>{logoUrl ? <Img src={logoUrl} style={{ maxWidth: 400, maxHeight: 400, objectFit: 'contain' }} /> : <div style={{ fontSize: 120, fontWeight: 900, color: '#f97316', fontFamily: 'Inter, system-ui, sans-serif', textShadow: '0 0 60px #f9731666' }}>LOGO</div>}</div>
    </AbsoluteFill>);
};
