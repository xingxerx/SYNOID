import { AbsoluteFill, useCurrentFrame, useVideoConfig, interpolate, spring } from 'remotion';
export interface CallToActionProps { type: 'subscribe' | 'like' | 'follow' | 'share' | 'custom'; customText?: string; style?: 'pill' | 'box' | 'floating' | 'pulse'; primaryColor?: string; position?: 'bottom-right' | 'bottom-left' | 'top-right' | 'top-left' | 'center'; }
const icons: Record<string, string> = { subscribe: '🔔', like: '👍', follow: '➕', share: '🔗', custom: '✨' };
const labels: Record<string, string> = { subscribe: 'Subscribe', like: 'Like', follow: 'Follow', share: 'Share', custom: '' };
export const CallToAction: React.FC<CallToActionProps> = ({ type, customText, style = 'pill', primaryColor = '#ef4444', position = 'bottom-right' }) => {
    const frame = useCurrentFrame(); const { fps, durationInFrames } = useVideoConfig();
    const text = customText || labels[type]; const icon = icons[type];
    const pos: Record<string, React.CSSProperties> = { 'bottom-right': { bottom: 40, right: 40 }, 'bottom-left': { bottom: 40, left: 40 }, 'top-right': { top: 40, right: 40 }, 'top-left': { top: 40, left: 40 }, center: { top: '50%', left: '50%', transform: 'translate(-50%,-50%)' } };
    const scaleIn = spring({ frame, fps, config: { damping: 12, stiffness: 200, mass: 0.8 } });
    const scaleOut = frame > durationInFrames - 15 ? interpolate(frame, [durationInFrames - 15, durationInFrames], [1, 0], { extrapolateRight: 'clamp' }) : 1;
    const scale = scaleIn * scaleOut;
    const pulse = style === 'pulse' ? interpolate(Math.sin(frame * 0.2), [-1, 1], [1, 1.1]) : 1;
    return (<AbsoluteFill><div style={{ position: 'absolute', ...pos[position], transform: `scale(${scale})` }}>
        <div style={{ backgroundColor: primaryColor, padding: style === 'box' ? 20 : '12px 24px', borderRadius: style === 'box' ? 12 : 50, display: 'flex', flexDirection: style === 'box' ? 'column' : 'row', alignItems: 'center', gap: 10, boxShadow: '0 4px 20px rgba(0,0,0,0.3)', transform: `scale(${pulse})` }}>
            <span style={{ fontSize: style === 'box' ? 36 : 24 }}>{icon}</span>
            <span style={{ color: 'white', fontSize: style === 'box' ? 14 : 20, fontWeight: 'bold', fontFamily: 'Inter, system-ui, sans-serif', textTransform: 'uppercase', letterSpacing: 1 }}>{text}</span>
        </div></div></AbsoluteFill>);
};
