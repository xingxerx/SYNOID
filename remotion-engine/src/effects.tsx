import React, { useMemo } from 'react';
import { AbsoluteFill, useCurrentFrame, useVideoConfig, interpolate, spring } from 'remotion';

// Particle component for explosion effects
export const Particle: React.FC<{
    delay: number; angle: number; distance: number; size: number; color: string; duration: number;
}> = ({ delay, angle, distance, size, color }) => {
    const frame = useCurrentFrame();
    const { fps } = useVideoConfig();
    const progress = spring({ frame: frame - delay, fps, config: { damping: 20, stiffness: 100, mass: 0.5 } });
    const x = Math.cos(angle) * distance * progress;
    const y = Math.sin(angle) * distance * progress;
    const opacity = interpolate(progress, [0, 0.3, 1], [0, 1, 0], { extrapolateRight: 'clamp' });
    const scale = interpolate(progress, [0, 0.2, 1], [0, 1.5, 0.3], { extrapolateRight: 'clamp' });
    return (
        <div style={{ position: 'absolute', width: size, height: size, borderRadius: '50%', backgroundColor: color, transform: `translate(${x}px, ${y}px) scale(${scale})`, opacity, boxShadow: `0 0 ${size * 2}px ${color}` }} />
    );
};

export const ExplosionEffect: React.FC<{ color?: string; particleCount?: number; delay?: number }> = ({ color = '#f97316', particleCount = 12, delay = 0 }) => {
    const particles = useMemo(() => Array.from({ length: particleCount }, (_, i) => ({
        angle: (i / particleCount) * Math.PI * 2, distance: 150 + Math.random() * 100, size: 8 + Math.random() * 12, delay: delay + Math.random() * 5,
    })), [particleCount, delay]);
    return (
        <div style={{ position: 'absolute', top: '50%', left: '50%' }}>
            {particles.map((p, i) => <Particle key={i} angle={p.angle} distance={p.distance} size={p.size} color={color} delay={p.delay} duration={30} />)}
        </div>
    );
};

export const GradientBackground: React.FC<{ color1?: string; color2?: string; color3?: string }> = ({ color1 = '#0a0a0a', color2 = '#1a1a2e', color3 = '#16213e' }) => {
    const frame = useCurrentFrame();
    const rotation = interpolate(frame, [0, 300], [0, 360], { extrapolateRight: 'extend' });
    return <div style={{ position: 'absolute', inset: -100, background: `conic-gradient(from ${rotation}deg at 50% 50%, ${color1}, ${color2}, ${color3}, ${color1})`, filter: 'blur(80px)', opacity: 0.6 }} />;
};

export const GlowingOrb: React.FC<{ x: number; y: number; size: number; color: string; delay?: number }> = ({ x, y, size, color, delay = 0 }) => {
    const frame = useCurrentFrame();
    const { fps } = useVideoConfig();
    const pulse = spring({ frame: frame - delay, fps, config: { damping: 5, stiffness: 20 }, durationInFrames: 60 });
    const scale = interpolate(Math.sin(frame * 0.1), [-1, 1], [0.8, 1.2]);
    return <div style={{ position: 'absolute', left: `${x}%`, top: `${y}%`, width: size, height: size, borderRadius: '50%', background: `radial-gradient(circle, ${color}88, ${color}00)`, transform: `scale(${scale * pulse})`, filter: `blur(${size / 4}px)` }} />;
};

export const AnimatedText: React.FC<{
    text: string; fontSize: number; color: string; delay?: number; style?: 'typewriter' | 'bounce' | 'wave' | 'glitch'; fontWeight?: string | number;
}> = ({ text, fontSize, color, delay = 0, style = 'bounce', fontWeight = 'bold' }) => {
    const frame = useCurrentFrame();
    const { fps, durationInFrames } = useVideoConfig();
    const characters = text.split('');
    const maxStaggerTime = Math.min(durationInFrames * 0.3, 40);
    const charStagger = characters.length > 1 ? Math.min(2, maxStaggerTime / characters.length) : 0;
    const scaledDelay = durationInFrames < 90 ? Math.min(delay, durationInFrames * 0.15) : delay;

    return (
        <div style={{ display: 'flex', justifyContent: 'center', flexWrap: 'wrap' }}>
            {characters.map((char, i) => {
                const charDelay = scaledDelay + i * charStagger;
                let transform = '', opacity = 1, charColor = color;
                if (style === 'bounce') {
                    const bounce = spring({ frame: frame - charDelay, fps, config: { damping: 8, stiffness: 200, mass: 0.5 } });
                    opacity = interpolate(bounce, [0, 0.5], [0, 1], { extrapolateRight: 'clamp' });
                    transform = `translateY(${interpolate(bounce, [0, 1], [30, 0])}px)`;
                } else if (style === 'wave') {
                    const wave = Math.sin((frame - charDelay) * 0.15) * 10;
                    opacity = interpolate(frame - charDelay, [0, 10], [0, 1], { extrapolateLeft: 'clamp', extrapolateRight: 'clamp' });
                    transform = `translateY(${wave}px)`;
                } else if (style === 'glitch') {
                    const gx = frame % 10 === 0 ? (Math.random() - 0.5) * 10 : 0;
                    opacity = interpolate(frame - charDelay, [0, 5], [0, 1], { extrapolateLeft: 'clamp', extrapolateRight: 'clamp' });
                    transform = `translate(${gx}px, 0)`;
                    if (frame % 20 < 2) charColor = '#00ffff';
                } else if (style === 'typewriter') {
                    opacity = frame > charDelay ? 1 : 0;
                }
                return (
                    <span key={i} style={{ display: 'inline-block', fontSize, fontWeight, color: charColor, fontFamily: 'Inter, system-ui, sans-serif', transform, opacity, textShadow: `0 0 20px ${color}66, 0 0 40px ${color}33`, whiteSpace: char === ' ' ? 'pre' : 'normal' }}>{char}</span>
                );
            })}
        </div>
    );
};

export const AnimatedNumber: React.FC<{
    value: number; prefix?: string; suffix?: string; fontSize?: number; color?: string; delay?: number; duration?: number;
}> = ({ value, prefix = '', suffix = '', fontSize = 108, color = '#f97316', delay = 0, duration = 60 }) => {
    const frame = useCurrentFrame();
    const targetValue = typeof value === 'number' && !isNaN(value) ? value : 0;
    const animatedFrame = Math.max(0, frame - delay);
    const progress = interpolate(animatedFrame, [0, duration], [0, 1], { extrapolateRight: 'clamp' });
    const easedProgress = 1 - Math.pow(1 - progress, 3);
    const currentValue = Math.floor(easedProgress * targetValue);
    const glowPulse = Math.sin((frame - delay) * 0.08) * 10 + 30;
    return (
        <div style={{ fontSize, fontWeight: 900, color, fontFamily: 'Inter, system-ui, sans-serif', textShadow: `0 0 ${glowPulse}px ${color}, 0 0 ${glowPulse * 2}px ${color}66`, letterSpacing: '-0.02em' }}>
            {prefix}{currentValue.toLocaleString()}{suffix}
        </div>
    );
};

export const CameraWrapper: React.FC<{
    children: React.ReactNode; type: string; intensity?: number;
}> = ({ children, type, intensity = 0.3 }) => {
    const frame = useCurrentFrame();
    const { durationInFrames, fps } = useVideoConfig();
    const progress = interpolate(frame, [0, durationInFrames], [0, 1], { extrapolateRight: 'clamp' });
    const smoothProgress = spring({ frame, fps, config: { damping: 100, stiffness: 50 }, durationInFrames });
    let transform = '';
    const maxMove = intensity * 100, maxZoom = 1 + intensity * 0.5;
    switch (type) {
        case 'zoom-in': transform = `scale(${interpolate(smoothProgress, [0, 1], [1, maxZoom])})`; break;
        case 'zoom-out': transform = `scale(${interpolate(smoothProgress, [0, 1], [maxZoom, 1])})`; break;
        case 'pan-left': transform = `translateX(${interpolate(progress, [0, 1], [maxMove, -maxMove])}px)`; break;
        case 'pan-right': transform = `translateX(${interpolate(progress, [0, 1], [-maxMove, maxMove])}px)`; break;
        case 'ken-burns': { const s = interpolate(smoothProgress, [0, 1], [1, maxZoom]); const x = interpolate(progress, [0, 1], [-maxMove * 0.3, maxMove * 0.3]); transform = `scale(${s}) translate(${x}px, 0)`; break; }
        default: transform = 'none';
    }
    return <div style={{ width: '100%', height: '100%', overflow: 'hidden' }}><div style={{ width: '100%', height: '100%', transform, transformOrigin: 'center center' }}>{children}</div></div>;
};

export const TransitionWrapper: React.FC<{
    children: React.ReactNode; transitionType: string; transitionDuration?: number;
}> = ({ children, transitionType, transitionDuration = 15 }) => {
    const frame = useCurrentFrame();
    const { durationInFrames, fps, width, height } = useVideoConfig();
    const exitStart = durationInFrames - transitionDuration;
    const entryProgress = interpolate(frame, [0, transitionDuration], [0, 1], { extrapolateLeft: 'clamp', extrapolateRight: 'clamp' });
    const exitProgress = interpolate(frame, [exitStart, durationInFrames], [0, 1], { extrapolateLeft: 'clamp', extrapolateRight: 'clamp' });
    const easedEntry = spring({ frame, fps, config: { damping: 15, stiffness: 100 }, durationInFrames: transitionDuration });
    const easedExit = spring({ frame: frame - exitStart, fps, config: { damping: 15, stiffness: 100 }, durationInFrames: transitionDuration });
    let style: React.CSSProperties = { width: '100%', height: '100%', position: 'absolute', top: 0, left: 0 };
    let opacity = 1; let transform: string | undefined; let clipPath: string | undefined; let filter: string | undefined;
    switch (transitionType) {
        case 'fade': opacity = interpolate(entryProgress, [0, 1], [0, 1]) * interpolate(exitProgress, [0, 1], [1, 0]); break;
        case 'swipe-left': transform = `translateX(${frame < exitStart ? interpolate(easedEntry, [0, 1], [width, 0]) : interpolate(easedExit, [0, 1], [0, -width])}px)`; break;
        case 'swipe-right': transform = `translateX(${frame < exitStart ? interpolate(easedEntry, [0, 1], [-width, 0]) : interpolate(easedExit, [0, 1], [0, width])}px)`; break;
        case 'zoom-in': { const s = frame < exitStart ? interpolate(easedEntry, [0, 1], [0.3, 1]) : interpolate(easedExit, [0, 1], [1, 1.5]); opacity = frame < exitStart ? entryProgress : (1 - exitProgress); transform = `scale(${s})`; break; }
        case 'blur': { const b = frame < exitStart ? interpolate(entryProgress, [0, 1], [20, 0]) : interpolate(exitProgress, [0, 1], [0, 20]); filter = `blur(${b}px)`; opacity = frame < exitStart ? entryProgress : (1 - exitProgress); break; }
        case 'wipe-left': clipPath = frame < exitStart ? `inset(0 ${interpolate(entryProgress, [0, 1], [100, 0])}% 0 0)` : `inset(0 0 0 ${interpolate(exitProgress, [0, 1], [0, 100])}%)`; break;
        default: break;
    }
    return <div style={{ ...style, transform, opacity, clipPath, filter, transformOrigin: 'center center' }}>{children}</div>;
};

export const SceneTransition: React.FC<{ type: string; color?: string; entering?: boolean }> = ({ type, color = '#f97316', entering = true }) => {
    const frame = useCurrentFrame();
    const { durationInFrames, fps } = useVideoConfig();
    const transitionFrames = 15;
    const progress = entering ? interpolate(frame, [0, transitionFrames], [0, 1], { extrapolateRight: 'clamp' }) : interpolate(frame, [durationInFrames - transitionFrames, durationInFrames], [0, 1], { extrapolateRight: 'clamp' });
    if (type === 'burst') {
        const scale = spring({ frame: entering ? frame : durationInFrames - frame, fps, config: { damping: 12, stiffness: 100 } });
        return <AbsoluteFill style={{ justifyContent: 'center', alignItems: 'center', pointerEvents: 'none' }}><div style={{ width: 50, height: 50, borderRadius: '50%', backgroundColor: color, transform: `scale(${entering ? (1 - scale) * 50 : scale * 50})`, opacity: entering ? 1 - progress : progress }} /></AbsoluteFill>;
    }
    return null;
};

export const ProgressBarChart: React.FC<{ value: number; maxValue: number; label: string; color?: string; delay?: number }> = ({ value, maxValue, label, color = '#f97316', delay = 0 }) => {
    const frame = useCurrentFrame();
    const { fps } = useVideoConfig();
    const progress = spring({ frame: frame - delay, fps, config: { damping: 15, stiffness: 100 } });
    const barWidth = interpolate(progress, [0, 1], [0, (value / maxValue) * 100]);
    const displayValue = Math.floor(interpolate(progress, [0, 1], [0, value]));
    return (
        <div style={{ width: '100%', marginBottom: 24 }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 8 }}>
                <span style={{ fontSize: 20, color: '#ffffff', fontFamily: 'Inter, system-ui, sans-serif' }}>{label}</span>
                <span style={{ fontSize: 20, color, fontWeight: 'bold', fontFamily: 'Inter, system-ui, sans-serif' }}>{displayValue}%</span>
            </div>
            <div style={{ width: '100%', height: 16, backgroundColor: '#333', borderRadius: 8, overflow: 'hidden' }}>
                <div style={{ width: `${barWidth}%`, height: '100%', backgroundColor: color, borderRadius: 8, boxShadow: `0 0 20px ${color}` }} />
            </div>
        </div>
    );
};

export const BarChart: React.FC<{ data: Array<{ label: string; value: number; color?: string }>; maxValue?: number; delay?: number }> = ({ data, maxValue, delay = 0 }) => {
    const frame = useCurrentFrame();
    const { fps } = useVideoConfig();
    const actualMax = maxValue || Math.max(...data.map(d => d.value));
    const colors = ['#f97316', '#3b82f6', '#22c55e', '#8b5cf6', '#ec4899', '#eab308'];
    return (
        <div style={{ display: 'flex', alignItems: 'flex-end', gap: 24, height: 300, padding: '0 40px' }}>
            {data.map((item, index) => {
                const itemDelay = delay + index * 8;
                const progress = spring({ frame: frame - itemDelay, fps, config: { damping: 12, stiffness: 100 } });
                const barHeight = interpolate(progress, [0, 1], [0, (item.value / actualMax) * 250]);
                const barColor = item.color || colors[index % colors.length];
                return (
                    <div key={index} style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', flex: 1 }}>
                        <div style={{ fontSize: 24, fontWeight: 'bold', color: barColor, marginBottom: 8, opacity: interpolate(progress, [0, 0.5, 1], [0, 1, 1]) }}>{item.value}</div>
                        <div style={{ width: '100%', maxWidth: 80, height: barHeight, backgroundColor: barColor, borderRadius: '8px 8px 0 0', boxShadow: `0 0 30px ${barColor}66` }} />
                        <div style={{ marginTop: 12, fontSize: 16, color: '#a1a1aa', textAlign: 'center', fontFamily: 'Inter, system-ui, sans-serif' }}>{item.label}</div>
                    </div>
                );
            })}
        </div>
    );
};

export const PieChart: React.FC<{ data: Array<{ label: string; value: number; color?: string }>; size?: number; delay?: number }> = ({ data, size = 300, delay = 0 }) => {
    const frame = useCurrentFrame();
    const { fps } = useVideoConfig();
    const colors = ['#f97316', '#3b82f6', '#22c55e', '#8b5cf6', '#ec4899', '#eab308'];
    const total = data.reduce((sum, d) => sum + d.value, 0);
    const progress = spring({ frame: frame - delay, fps, config: { damping: 15, stiffness: 80 } });
    let gradientStops = '', angleAcc = 0;
    data.forEach((item, i) => {
        const angle = (item.value / total) * 360 * progress;
        const startPct = (angleAcc / 360) * 100;
        angleAcc += angle;
        const endPct = (angleAcc / 360) * 100;
        const c = item.color || colors[i % colors.length];
        gradientStops += `${c} ${startPct}% ${endPct}%${i < data.length - 1 ? ', ' : ''}`;
    });
    return (
        <div style={{ display: 'flex', alignItems: 'center', gap: 40 }}>
            <div style={{ width: size, height: size, borderRadius: '50%', background: `conic-gradient(from -90deg, ${gradientStops || '#333 0% 100%'})`, boxShadow: '0 0 60px rgba(0,0,0,0.5)' }} />
            <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
                {data.map((item, i) => (
                    <div key={i} style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
                        <div style={{ width: 16, height: 16, borderRadius: 4, backgroundColor: item.color || colors[i % colors.length] }} />
                        <span style={{ color: '#ffffff', fontSize: 18, fontFamily: 'Inter, system-ui, sans-serif' }}>{item.label}: {item.value}</span>
                    </div>
                ))}
            </div>
        </div>
    );
};

export const extractNumericFromString = (valueStr: string): { numericValue: number; prefix: string; suffix: string } | null => {
    if (!valueStr || typeof valueStr !== 'string') return null;
    const str = valueStr.trim();
    let prefix = '';
    const prefixMatch = str.match(/^([£$€¥₹#@~]+)/);
    if (prefixMatch) prefix = prefixMatch[1];
    const numberMatch = str.match(/[\d,]+\.?\d*/);
    if (!numberMatch || numberMatch[0] === '') return null;
    let numericValue = parseFloat(numberMatch[0].replace(/,/g, ''));
    if (isNaN(numericValue)) return null;
    const numberEndIndex = str.indexOf(numberMatch[0]) + numberMatch[0].length;
    const afterNumber = str.substring(numberEndIndex).trim();
    let suffix = '';
    if (/^k\b/i.test(afterNumber)) { numericValue *= 1000; suffix = afterNumber.replace(/^k\b/i, '').trim(); }
    else if (/^m\b/i.test(afterNumber)) { numericValue *= 1000000; suffix = afterNumber.replace(/^m\b/i, '').trim(); }
    else { suffix = afterNumber; }
    return { numericValue: Math.round(numericValue), prefix, suffix };
};
