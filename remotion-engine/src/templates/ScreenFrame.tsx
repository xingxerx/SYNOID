import { AbsoluteFill, useCurrentFrame, useVideoConfig, interpolate, spring, Img } from 'remotion';
export interface ScreenFrameProps { device?: 'macbook' | 'iphone' | 'browser' | 'minimal'; screenshotUrl: string; }
export const ScreenFrame: React.FC<ScreenFrameProps> = ({ device = 'macbook', screenshotUrl }) => {
    const frame = useCurrentFrame(); const { fps } = useVideoConfig();
    const scale = spring({ frame, fps, config: { damping: 12, stiffness: 80 } });
    const br = device === 'iphone' ? 40 : 12; const ar = device === 'iphone' ? '9/19' : '16/10';
    return (<AbsoluteFill style={{ justifyContent: 'center', alignItems: 'center', display: 'flex', backgroundColor: '#0a0a0a' }}>
        <div style={{ transform: `scale(${scale})`, maxWidth: device === 'iphone' ? 350 : 900, width: '100%' }}>
            {device !== 'minimal' && <div style={{ backgroundColor: '#1a1a1a', padding: '8px 16px', borderRadius: `${br}px ${br}px 0 0`, display: 'flex', gap: 8, alignItems: 'center' }}>
                <div style={{ width: 12, height: 12, borderRadius: '50%', backgroundColor: '#ff5f57' }} />
                <div style={{ width: 12, height: 12, borderRadius: '50%', backgroundColor: '#febc2e' }} />
                <div style={{ width: 12, height: 12, borderRadius: '50%', backgroundColor: '#28c840' }} />
            </div>}
            <div style={{ aspectRatio: ar, overflow: 'hidden', borderRadius: device === 'minimal' ? br : `0 0 ${br}px ${br}px`, backgroundColor: '#111', boxShadow: '0 25px 80px rgba(0,0,0,0.6)' }}>
                {screenshotUrl ? <Img src={screenshotUrl} style={{ width: '100%', height: '100%', objectFit: 'cover' }} /> : <div style={{ width: '100%', height: '100%', display: 'flex', justifyContent: 'center', alignItems: 'center', color: '#555', fontSize: 24 }}>Screenshot</div>}
            </div>
        </div>
    </AbsoluteFill>);
};
