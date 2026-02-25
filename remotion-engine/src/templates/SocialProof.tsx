import { AbsoluteFill, useCurrentFrame, useVideoConfig, interpolate, spring } from 'remotion';
export interface SocialProofProps { quote: string; author?: string; rating?: number; style?: 'card' | 'minimal' | 'tweet' | 'review'; avatarUrl?: string; }
export const SocialProof: React.FC<SocialProofProps> = ({ quote, author, rating = 5, style = 'card', avatarUrl }) => {
    const frame = useCurrentFrame(); const { fps } = useVideoConfig();
    const scale = spring({ frame, fps, config: { damping: 12, stiffness: 100 } });
    const opacity = interpolate(frame, [0, 15], [0, 1], { extrapolateRight: 'clamp' });
    return (<AbsoluteFill style={{ justifyContent: 'center', alignItems: 'center', display: 'flex' }}>
        <div style={{ maxWidth: 700, padding: 48, backgroundColor: style === 'card' ? 'rgba(255,255,255,0.05)' : 'transparent', borderRadius: 24, border: style === 'card' ? '1px solid rgba(255,255,255,0.1)' : undefined, transform: `scale(${scale})`, opacity, backdropFilter: 'blur(10px)' }}>
            <div style={{ fontSize: 20, color: '#fbbf24', marginBottom: 20 }}>{'⭐'.repeat(rating)}</div>
            <div style={{ fontSize: 32, color: '#fff', fontFamily: 'Inter, system-ui, sans-serif', lineHeight: 1.5, fontStyle: 'italic', fontWeight: 300 }}>"{quote}"</div>
            {author && <div style={{ marginTop: 24, fontSize: 18, color: '#a1a1aa', fontFamily: 'Inter, system-ui, sans-serif', fontWeight: 600 }}>— {author}</div>}
        </div>
    </AbsoluteFill>);
};
