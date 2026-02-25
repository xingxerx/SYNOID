import { AbsoluteFill, useCurrentFrame, useVideoConfig, interpolate } from 'remotion';
export interface CounterProps { from?: number; to: number; prefix?: string; suffix?: string; label?: string; color?: string; fontSize?: number; }
export const Counter: React.FC<CounterProps> = ({ from = 0, to, prefix = '', suffix = '', label, color = '#f97316', fontSize = 96 }) => {
    const frame = useCurrentFrame(); const { durationInFrames } = useVideoConfig();
    const progress = interpolate(frame, [0, durationInFrames * 0.7], [0, 1], { extrapolateRight: 'clamp' });
    const eased = 1 - Math.pow(1 - progress, 3);
    const value = Math.floor(from + (to - from) * eased);
    return (<AbsoluteFill style={{ justifyContent: 'center', alignItems: 'center', display: 'flex', flexDirection: 'column', gap: 20 }}>
        <div style={{ fontSize, fontWeight: 900, color, fontFamily: 'Inter, system-ui, sans-serif', textShadow: `0 0 40px ${color}` }}>{prefix}{value.toLocaleString()}{suffix}</div>
        {label && <div style={{ fontSize: 24, color: '#a1a1aa', textTransform: 'uppercase', letterSpacing: '0.15em', fontFamily: 'Inter, system-ui, sans-serif' }}>{label}</div>}
    </AbsoluteFill>);
};
