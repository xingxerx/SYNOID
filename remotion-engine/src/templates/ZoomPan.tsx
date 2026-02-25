import { AbsoluteFill, useCurrentFrame, useVideoConfig, interpolate, Img } from 'remotion';
export interface ZoomPanProps { imageUrl: string; style?: 'zoom-in' | 'zoom-out' | 'pan-left' | 'pan-right' | 'ken-burns'; duration?: number; }
export const ZoomPan: React.FC<ZoomPanProps> = ({ imageUrl, style = 'zoom-in' }) => {
    const frame = useCurrentFrame(); const { durationInFrames } = useVideoConfig();
    const progress = interpolate(frame, [0, durationInFrames], [0, 1], { extrapolateRight: 'clamp' });
    let transform = '';
    switch (style) {
        case 'zoom-in': transform = `scale(${interpolate(progress, [0, 1], [1, 1.3])})`; break;
        case 'zoom-out': transform = `scale(${interpolate(progress, [0, 1], [1.3, 1])})`; break;
        case 'pan-left': transform = `translateX(${interpolate(progress, [0, 1], [50, -50])}px) scale(1.1)`; break;
        case 'pan-right': transform = `translateX(${interpolate(progress, [0, 1], [-50, 50])}px) scale(1.1)`; break;
        case 'ken-burns': { const s = interpolate(progress, [0, 1], [1, 1.2]); const x = interpolate(progress, [0, 1], [-20, 20]); transform = `scale(${s}) translateX(${x}px)`; break; }
    }
    return (<AbsoluteFill style={{ overflow: 'hidden' }}>
        <div style={{ width: '100%', height: '100%', transform, transformOrigin: 'center center' }}>
            {imageUrl ? <Img src={imageUrl} style={{ width: '100%', height: '100%', objectFit: 'cover' }} /> : <div style={{ width: '100%', height: '100%', backgroundColor: '#1a1a1a' }} />}
        </div>
    </AbsoluteFill>);
};
