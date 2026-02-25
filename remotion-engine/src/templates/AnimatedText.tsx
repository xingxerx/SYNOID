import { AbsoluteFill, useCurrentFrame, useVideoConfig, interpolate, spring } from 'remotion';

export interface AnimatedTextProps {
    text: string;
    style?: 'typewriter' | 'bounce' | 'fade-up' | 'word-by-word' | 'glitch';
    color?: string;
    fontSize?: number;
    fontFamily?: string;
    backgroundColor?: string;
    position?: 'center' | 'bottom' | 'top';
}

export const AnimatedText: React.FC<AnimatedTextProps> = ({
    text, style = 'typewriter', color = '#ffffff', fontSize = 64,
    fontFamily = 'Inter, system-ui, sans-serif', backgroundColor = 'transparent', position = 'center',
}) => {
    const frame = useCurrentFrame();
    const { fps, durationInFrames } = useVideoConfig();
    const positionStyles: React.CSSProperties = { center: { justifyContent: 'center', alignItems: 'center' }, bottom: { justifyContent: 'flex-end', alignItems: 'center', paddingBottom: 80 }, top: { justifyContent: 'flex-start', alignItems: 'center', paddingTop: 80 } }[position];

    if (style === 'typewriter') {
        const charsToShow = Math.floor(interpolate(frame, [0, durationInFrames * 0.7], [0, text.length], { extrapolateRight: 'clamp' }));
        const showCursor = frame % Math.floor(fps / 2) < fps / 4;
        return (<AbsoluteFill style={{ backgroundColor, ...positionStyles, display: 'flex' }}><div style={{ color, fontSize, fontFamily, fontWeight: 'bold', textShadow: '2px 2px 8px rgba(0,0,0,0.5)' }}>{text.slice(0, charsToShow)}<span style={{ opacity: showCursor ? 1 : 0 }}>|</span></div></AbsoluteFill>);
    }

    if (style === 'bounce') {
        const scale = spring({ frame, fps, config: { damping: 10, stiffness: 100, mass: 0.5 } });
        return (<AbsoluteFill style={{ backgroundColor, ...positionStyles, display: 'flex' }}><div style={{ color, fontSize, fontFamily, fontWeight: 'bold', transform: `scale(${scale})`, textShadow: '2px 2px 8px rgba(0,0,0,0.5)' }}>{text}</div></AbsoluteFill>);
    }

    if (style === 'fade-up') {
        const opacity = interpolate(frame, [0, 20], [0, 1], { extrapolateRight: 'clamp' });
        const translateY = interpolate(frame, [0, 20], [30, 0], { extrapolateRight: 'clamp' });
        return (<AbsoluteFill style={{ backgroundColor, ...positionStyles, display: 'flex' }}><div style={{ color, fontSize, fontFamily, fontWeight: 'bold', opacity, transform: `translateY(${translateY}px)`, textShadow: '2px 2px 8px rgba(0,0,0,0.5)' }}>{text}</div></AbsoluteFill>);
    }

    if (style === 'word-by-word') {
        const words = text.split(' ');
        const framesPerWord = Math.floor(durationInFrames * 0.7 / words.length);
        return (<AbsoluteFill style={{ backgroundColor, ...positionStyles, display: 'flex' }}><div style={{ display: 'flex', gap: fontSize * 0.3, flexWrap: 'wrap', justifyContent: 'center', maxWidth: '80%' }}>
            {words.map((word, i) => {
                const ws = i * framesPerWord;
                const opacity = interpolate(frame, [ws, ws + 10], [0, 1], { extrapolateLeft: 'clamp', extrapolateRight: 'clamp' });
                const s = spring({ frame: Math.max(0, frame - ws), fps, config: { damping: 12, stiffness: 200 } });
                return <span key={i} style={{ color, fontSize, fontFamily, fontWeight: 'bold', opacity, transform: `scale(${s})`, textShadow: '2px 2px 8px rgba(0,0,0,0.5)' }}>{word}</span>;
            })}
        </div></AbsoluteFill>);
    }

    if (style === 'glitch') {
        const glitchOffset = Math.sin(frame * 0.5) * 3;
        const showGlitch = frame % 15 < 3;
        return (<AbsoluteFill style={{ backgroundColor, ...positionStyles, display: 'flex' }}><div style={{ position: 'relative' }}>
            {showGlitch && <><div style={{ position: 'absolute', color: '#ff0000', fontSize, fontFamily, fontWeight: 'bold', transform: `translate(${glitchOffset}px, ${-glitchOffset}px)`, opacity: 0.7 }}>{text}</div><div style={{ position: 'absolute', color: '#00ffff', fontSize, fontFamily, fontWeight: 'bold', transform: `translate(${-glitchOffset}px, ${glitchOffset}px)`, opacity: 0.7 }}>{text}</div></>}
            <div style={{ color, fontSize, fontFamily, fontWeight: 'bold', textShadow: '2px 2px 8px rgba(0,0,0,0.5)', position: 'relative' }}>{text}</div>
        </div></AbsoluteFill>);
    }

    return <AbsoluteFill style={{ backgroundColor, ...positionStyles, display: 'flex' }}><div style={{ color, fontSize, fontFamily, fontWeight: 'bold' }}>{text}</div></AbsoluteFill>;
};
