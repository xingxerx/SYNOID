import { AbsoluteFill, useCurrentFrame, useVideoConfig, interpolate, spring, Img } from 'remotion';
export interface ComparisonProps { type?: 'slider' | 'side-by-side' | 'flip' | 'fade'; beforeLabel?: string; afterLabel?: string; beforeImageUrl?: string; afterImageUrl?: string; beforeColor?: string; afterColor?: string; style?: 'minimal' | 'labeled' | 'dramatic'; }
export const Comparison: React.FC<ComparisonProps> = ({ type = 'slider', beforeLabel = 'Before', afterLabel = 'After', beforeImageUrl, afterImageUrl, beforeColor = '#ef4444', afterColor = '#22c55e', style = 'labeled' }) => {
    const frame = useCurrentFrame(); const { fps, durationInFrames } = useVideoConfig();
    const opacity = interpolate(frame, [0, 20], [0, 1], { extrapolateRight: 'clamp' });
    if (type === 'slider') {
        const sliderPosition = interpolate(frame, [30, durationInFrames * 0.7], [10, 90], { extrapolateLeft: 'clamp', extrapolateRight: 'clamp' });
        return (<AbsoluteFill style={{ justifyContent: 'center', alignItems: 'center', display: 'flex' }}><div style={{ width: '80%', maxWidth: 1200, aspectRatio: '16/9', position: 'relative', overflow: 'hidden', borderRadius: 16, opacity, boxShadow: '0 25px 50px -12px rgba(0,0,0,0.5)' }}>
            <div style={{ position: 'absolute', inset: 0, backgroundColor: afterColor }}>{afterImageUrl ? <Img src={afterImageUrl} style={{ width: '100%', height: '100%', objectFit: 'cover' }} /> : <div style={{ width: '100%', height: '100%', display: 'flex', justifyContent: 'center', alignItems: 'center' }}><span style={{ color: '#fff', fontSize: 48, fontWeight: 'bold' }}>{afterLabel}</span></div>}</div>
            <div style={{ position: 'absolute', inset: 0, clipPath: `inset(0 ${100 - sliderPosition}% 0 0)`, backgroundColor: beforeColor }}>{beforeImageUrl ? <Img src={beforeImageUrl} style={{ width: '100%', height: '100%', objectFit: 'cover' }} /> : <div style={{ width: '100%', height: '100%', display: 'flex', justifyContent: 'center', alignItems: 'center' }}><span style={{ color: '#fff', fontSize: 48, fontWeight: 'bold' }}>{beforeLabel}</span></div>}</div>
            <div style={{ position: 'absolute', top: 0, bottom: 0, left: `${sliderPosition}%`, width: 4, backgroundColor: '#fff', boxShadow: '0 0 20px rgba(0,0,0,0.5)' }}><div style={{ position: 'absolute', top: '50%', left: '50%', transform: 'translate(-50%,-50%)', width: 50, height: 50, borderRadius: '50%', backgroundColor: '#fff', display: 'flex', justifyContent: 'center', alignItems: 'center', boxShadow: '0 4px 12px rgba(0,0,0,0.3)' }}><span style={{ fontSize: 20 }}>⟷</span></div></div>
        </div></AbsoluteFill>);
    }
    const leftScale = spring({ frame, fps, config: { damping: 15, stiffness: 100 } });
    const rightScale = spring({ frame: Math.max(0, frame - 15), fps, config: { damping: 15, stiffness: 100 } });
    return (<AbsoluteFill style={{ justifyContent: 'center', alignItems: 'center', display: 'flex', gap: 40 }}>
        <div style={{ width: '40%', aspectRatio: '4/3', backgroundColor: beforeColor, borderRadius: 16, overflow: 'hidden', transform: `scale(${leftScale})`, opacity }}>{beforeImageUrl ? <Img src={beforeImageUrl} style={{ width: '100%', height: '100%', objectFit: 'cover' }} /> : <div style={{ width: '100%', height: '100%', display: 'flex', justifyContent: 'center', alignItems: 'center' }}><span style={{ color: '#fff', fontSize: 36, fontWeight: 'bold' }}>{beforeLabel}</span></div>}</div>
        <div style={{ fontSize: 32, fontWeight: 'bold', color: '#fff', opacity: interpolate(frame, [20, 40], [0, 1], { extrapolateRight: 'clamp' }) }}>VS</div>
        <div style={{ width: '40%', aspectRatio: '4/3', backgroundColor: afterColor, borderRadius: 16, overflow: 'hidden', transform: `scale(${rightScale})`, opacity }}>{afterImageUrl ? <Img src={afterImageUrl} style={{ width: '100%', height: '100%', objectFit: 'cover' }} /> : <div style={{ width: '100%', height: '100%', display: 'flex', justifyContent: 'center', alignItems: 'center' }}><span style={{ color: '#fff', fontSize: 36, fontWeight: 'bold' }}>{afterLabel}</span></div>}</div>
    </AbsoluteFill>);
};
