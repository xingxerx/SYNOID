import { AbsoluteFill, useCurrentFrame, useVideoConfig, interpolate, spring } from 'remotion';
export interface LowerThirdProps { name: string; title?: string; style?: 'modern' | 'minimal' | 'gradient' | 'glassmorphism'; primaryColor?: string; }
export const LowerThird: React.FC<LowerThirdProps> = ({ name, title, style = 'modern', primaryColor = '#f97316' }) => {
    const frame = useCurrentFrame(); const { fps, durationInFrames } = useVideoConfig();
    const slideIn = spring({ frame, fps, config: { damping: 15, stiffness: 100 } });
    const slideOut = frame > durationInFrames - 15 ? interpolate(frame, [durationInFrames - 15, durationInFrames], [0, -200], { extrapolateRight: 'clamp' }) : 0;
    const width = interpolate(slideIn, [0, 1], [0, 400]);
    return (<AbsoluteFill><div style={{ position: 'absolute', bottom: 80, left: 60, transform: `translateX(${slideOut}px)` }}>
        <div style={{ backgroundColor: style === 'glassmorphism' ? 'rgba(255,255,255,0.1)' : primaryColor, backdropFilter: style === 'glassmorphism' ? 'blur(20px)' : undefined, padding: '16px 32px', borderRadius: style === 'minimal' ? 0 : 8, width, overflow: 'hidden', boxShadow: style !== 'minimal' ? '0 4px 20px rgba(0,0,0,0.3)' : undefined, borderLeft: style === 'minimal' ? `4px solid ${primaryColor}` : undefined }}>
            <div style={{ fontSize: 28, fontWeight: 'bold', color: '#fff', fontFamily: 'Inter, system-ui, sans-serif', whiteSpace: 'nowrap' }}>{name}</div>
            {title && <div style={{ fontSize: 16, color: 'rgba(255,255,255,0.8)', fontFamily: 'Inter, system-ui, sans-serif', marginTop: 4 }}>{title}</div>}
        </div></div></AbsoluteFill>);
};
