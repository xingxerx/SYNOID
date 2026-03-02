import { AbsoluteFill, useCurrentFrame, useVideoConfig, interpolate, spring } from 'remotion';
export interface ProgressBarProps { value: number; maxValue?: number; label?: string; color?: string; style?: 'bar' | 'circle' | 'gauge'; }
export const ProgressBar: React.FC<ProgressBarProps> = ({ value, maxValue = 100, label, color = '#22c55e', style = 'bar' }) => {
    const frame = useCurrentFrame(); const { fps } = useVideoConfig();
    const progress = spring({ frame, fps, config: { damping: 15, stiffness: 100 } });
    const barWidth = interpolate(progress, [0, 1], [0, (value / maxValue) * 100]);
    const displayValue = Math.floor(interpolate(progress, [0, 1], [0, value]));
    return (<AbsoluteFill style={{ justifyContent: 'center', alignItems: 'center', display: 'flex', flexDirection: 'column', gap: 20 }}>
        {label && <div style={{ fontSize: 28, color: '#fff', fontFamily: 'Inter, system-ui, sans-serif', fontWeight: 600 }}>{label}</div>}
        {style === 'bar' && <div style={{ width: 600, height: 24, backgroundColor: '#333', borderRadius: 12, overflow: 'hidden' }}><div style={{ width: `${barWidth}%`, height: '100%', backgroundColor: color, borderRadius: 12, boxShadow: `0 0 30px ${color}` }} /></div>}
        <div style={{ fontSize: 48, fontWeight: 900, color, fontFamily: 'Inter, system-ui, sans-serif' }}>{displayValue}%</div>
    </AbsoluteFill>);
};
