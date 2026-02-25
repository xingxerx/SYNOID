import React, { useMemo, useEffect, useState } from 'react';
import { AbsoluteFill, useCurrentFrame, useVideoConfig, interpolate, spring, Img, OffthreadVideo, Sequence } from 'remotion';
import { preloadVideo, preloadImage } from '@remotion/preload';
import { Circle, Rect, Triangle, Star, Polygon, Ellipse } from '@remotion/shapes';
import { AnimatedEmoji } from '@remotion/animated-emoji';
import { Gif } from '@remotion/gif';
import { Lottie, LottieAnimationData } from '@remotion/lottie';
import { delayRender, continueRender } from 'remotion';
import { GradientBackground, GlowingOrb, ExplosionEffect, AnimatedText, AnimatedNumber, SceneTransition, BarChart, PieChart, ProgressBarChart, CameraWrapper, TransitionWrapper, extractNumericFromString } from './effects';
import { Scene3D } from './components/Scene3D';
import type { Scene } from './Root';

// ─── Title Scene ────────────────────────────────────────────
export const TitleScene: React.FC<{ content: Scene['content'] }> = ({ content }) => {
    const frame = useCurrentFrame();
    const { fps, durationInFrames } = useVideoConfig();
    const accentColor = content.color || '#f97316';
    const titleScale = spring({ frame, fps, config: { damping: 12, stiffness: 100 } });
    const isShort = durationInFrames < 90;
    const subtitleStart = isShort ? Math.round(durationInFrames * 0.15) : 20;
    const subtitleEnd = isShort ? Math.round(durationInFrames * 0.35) : 40;
    const subtitleOpacity = interpolate(frame, [subtitleStart, subtitleEnd], [0, 1], { extrapolateRight: 'clamp' });
    const exitFrames = isShort ? Math.round(durationInFrames * 0.2) : 20;
    const exitProgress = interpolate(frame, [durationInFrames - exitFrames, durationInFrames], [0, 1], { extrapolateRight: 'clamp' });
    const exitScale = interpolate(exitProgress, [0, 1], [1, 0.8]);
    const exitOpacity = interpolate(exitProgress, [0, 1], [1, 0]);
    return (
        <AbsoluteFill style={{ justifyContent: 'center', alignItems: 'center', backgroundColor: content.backgroundColor || 'transparent', overflow: 'hidden' }}>
            <GradientBackground color1="#0a0a0a" color2={accentColor + '33'} color3="#0a0a0a" />
            <GlowingOrb x={20} y={30} size={200} color={accentColor} delay={5} />
            <GlowingOrb x={80} y={70} size={150} color="#3b82f6" delay={10} />
            {frame < 30 && <ExplosionEffect color={accentColor} particleCount={16} delay={5} />}
            <div style={{ textAlign: 'center', transform: `scale(${titleScale * exitScale})`, opacity: exitOpacity, zIndex: 10 }}>
                {content.title && <AnimatedText text={content.title} fontSize={90} color={content.color || '#ffffff'} style="bounce" delay={0} />}
                {content.subtitle && <div style={{ marginTop: 30, opacity: subtitleOpacity }}><AnimatedText text={content.subtitle} fontSize={36} color="#a1a1aa" style="wave" delay={isShort ? Math.round(durationInFrames * 0.15) : 25} fontWeight={400} /></div>}
                <div style={{ width: interpolate(frame, [15, 35], [0, 300], { extrapolateRight: 'clamp' }), height: 4, backgroundColor: accentColor, margin: '30px auto 0', borderRadius: 2, boxShadow: `0 0 20px ${accentColor}` }} />
            </div>
            <SceneTransition type="burst" color={accentColor} entering />
        </AbsoluteFill>
    );
};

// ─── Steps Scene ────────────────────────────────────────────
export const StepsScene: React.FC<{ content: Scene['content'] }> = ({ content }) => {
    const frame = useCurrentFrame();
    const { fps, durationInFrames } = useVideoConfig();
    const items = content.items || [];
    const accentColor = content.color || '#f97316';
    return (
        <AbsoluteFill style={{ justifyContent: 'center', alignItems: 'center', backgroundColor: content.backgroundColor || 'transparent', padding: 80, overflow: 'hidden' }}>
            <GradientBackground color1="#0a0a0a" color2="#1a1a2e" color3={accentColor + '22'} />
            {content.title && <div style={{ position: 'absolute', top: 80 }}><AnimatedText text={content.title} fontSize={56} color={content.color || '#ffffff'} style="bounce" /></div>}
            <div style={{ display: 'flex', gap: 80, justifyContent: 'center', alignItems: 'flex-start', marginTop: content.title ? 80 : 0 }}>
                {items.map((item, index) => {
                    const stagger = durationInFrames < 90 ? Math.round(durationInFrames * 0.08) : 15;
                    const start = durationInFrames < 90 ? Math.round(durationInFrames * 0.1) : 20;
                    const itemSpring = spring({ frame: frame - (start + index * stagger), fps, config: { damping: 10, stiffness: 150, mass: 0.8 } });
                    const itemScale = interpolate(itemSpring, [0, 1], [0.5, 1]);
                    const itemOpacity = interpolate(itemSpring, [0, 0.5], [0, 1], { extrapolateRight: 'clamp' });
                    return (
                        <div key={index} style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 24, maxWidth: 320, opacity: itemOpacity, transform: `scale(${itemScale})` }}>
                            <div style={{ width: 100, height: 100, borderRadius: 24, background: `linear-gradient(135deg, ${accentColor}, ${accentColor}88)`, display: 'flex', justifyContent: 'center', alignItems: 'center', fontSize: 48, fontWeight: 'bold', color: '#ffffff', fontFamily: 'Inter, system-ui, sans-serif', boxShadow: `0 10px 40px ${accentColor}66` }}>{item.icon || index + 1}</div>
                            <div style={{ fontSize: 28, fontWeight: 'bold', color: '#ffffff', textAlign: 'center', fontFamily: 'Inter, system-ui, sans-serif' }}>{item.label}</div>
                            {item.description && <div style={{ fontSize: 18, color: '#a1a1aa', textAlign: 'center', fontFamily: 'Inter, system-ui, sans-serif', lineHeight: 1.5 }}>{item.description}</div>}
                        </div>
                    );
                })}
            </div>
        </AbsoluteFill>
    );
};

// ─── Stats Scene ────────────────────────────────────────────
export const StatsScene: React.FC<{ content: Scene['content'] }> = ({ content }) => {
    const frame = useCurrentFrame();
    const { fps, durationInFrames } = useVideoConfig();
    const stats = content.stats || [];
    const accentColor = content.color || '#f97316';
    const colors = [accentColor, '#3b82f6', '#22c55e', '#8b5cf6', '#ec4899'];
    const isShort = durationInFrames < 90;
    const processedStats = useMemo(() => stats.map((stat) => {
        if (typeof stat.numericValue === 'number' && !isNaN(stat.numericValue)) return stat;
        if (stat.value) { const extracted = extractNumericFromString(stat.value); if (extracted) return { ...stat, numericValue: extracted.numericValue, prefix: stat.prefix || extracted.prefix, suffix: stat.suffix || extracted.suffix }; }
        return stat;
    }), [stats]);

    return (
        <AbsoluteFill style={{ justifyContent: 'center', alignItems: 'center', backgroundColor: content.backgroundColor || 'transparent', overflow: 'hidden' }}>
            <GradientBackground color1="#0a0a0a" color2={accentColor + '22'} color3="#0f0f23" />
            {content.title && <div style={{ position: 'absolute', top: 100 }}><AnimatedText text={content.title} fontSize={52} color={content.color || '#ffffff'} style="bounce" /></div>}
            <div style={{ display: 'flex', gap: 120, justifyContent: 'center', flexWrap: 'wrap', maxWidth: 1400 }}>
                {processedStats.map((stat, index) => {
                    const delay = (isShort ? Math.round(durationInFrames * 0.1) : 15) + index * (isShort ? Math.round(durationInFrames * 0.08) : 12);
                    const statColor = colors[index % colors.length];
                    const entrySpring = spring({ frame: frame - delay, fps, config: { damping: 8, stiffness: 100, mass: 1 } });
                    const scale = interpolate(entrySpring, [0, 1], [0, 1]);
                    const hasNumericValue = typeof stat.numericValue === 'number' && !isNaN(stat.numericValue) && stat.numericValue > 0;
                    return (
                        <div key={index} style={{ textAlign: 'center', transform: `scale(${scale})`, minWidth: 200 }}>
                            {hasNumericValue ? <AnimatedNumber value={stat.numericValue!} prefix={stat.prefix} suffix={stat.suffix} fontSize={96} color={statColor} delay={delay} duration={isShort ? Math.round(durationInFrames * 0.5) : 75} /> : <div style={{ fontSize: 96, fontWeight: 900, color: statColor, fontFamily: 'Inter, system-ui, sans-serif', textShadow: `0 0 30px ${statColor}` }}>{stat.value}</div>}
                            <div style={{ fontSize: 22, color: '#a1a1aa', marginTop: 16, textTransform: 'uppercase', letterSpacing: '0.15em', fontFamily: 'Inter, system-ui, sans-serif', fontWeight: 500 }}>{stat.label}</div>
                        </div>
                    );
                })}
            </div>
        </AbsoluteFill>
    );
};

// ─── Text Scene ─────────────────────────────────────────────
export const TextScene: React.FC<{ content: Scene['content'] }> = ({ content }) => {
    const frame = useCurrentFrame();
    const { durationInFrames } = useVideoConfig();
    const accentColor = content.color || '#ffffff';
    const exitOpacity = interpolate(frame, [durationInFrames - 20, durationInFrames], [1, 0], { extrapolateRight: 'clamp' });
    return (
        <AbsoluteFill style={{ justifyContent: 'center', alignItems: 'center', backgroundColor: content.backgroundColor || 'transparent', padding: 120, overflow: 'hidden' }}>
            <GradientBackground />
            <div style={{ maxWidth: 1400, opacity: exitOpacity }}><AnimatedText text={content.title || ''} fontSize={56} color={accentColor} style="wave" fontWeight={600} /></div>
            <GlowingOrb x={10} y={20} size={100} color={accentColor} />
        </AbsoluteFill>
    );
};

// ─── Transition Scene ───────────────────────────────────────
export const TransitionScene: React.FC<{ content: Scene['content'] }> = ({ content }) => {
    const frame = useCurrentFrame();
    const { durationInFrames } = useVideoConfig();
    const accentColor = content.color || '#f97316';
    const rings = [0, 10, 20].map((delay) => {
        const p = interpolate(frame - delay, [0, durationInFrames - delay], [0, 1], { extrapolateLeft: 'clamp', extrapolateRight: 'clamp' });
        return { scale: interpolate(p, [0, 1], [0, 30]), opacity: interpolate(p, [0, 0.3, 1], [0.8, 0.5, 0]) };
    });
    return (
        <AbsoluteFill style={{ justifyContent: 'center', alignItems: 'center', backgroundColor: content.backgroundColor || '#0a0a0a', overflow: 'hidden' }}>
            {rings.map((ring, i) => <div key={i} style={{ position: 'absolute', width: 60, height: 60, borderRadius: '50%', border: `3px solid ${accentColor}`, transform: `scale(${ring.scale})`, opacity: ring.opacity, boxShadow: `0 0 30px ${accentColor}` }} />)}
            <ExplosionEffect color={accentColor} particleCount={20} delay={0} />
        </AbsoluteFill>
    );
};

// ─── Media Scene ────────────────────────────────────────────
export const MediaScene: React.FC<{ content: Scene['content'] }> = ({ content }) => {
    const frame = useCurrentFrame();
    const { fps, durationInFrames } = useVideoConfig();
    const accentColor = content.color || '#f97316';
    useEffect(() => { if (content.mediaPath) { try { if (content.mediaType === 'video') preloadVideo(content.mediaPath); else preloadImage(content.mediaPath); } catch { } } }, [content.mediaPath, content.mediaType]);
    const entrySpring = spring({ frame, fps, config: { damping: 12, stiffness: 100 } });
    const scale = interpolate(entrySpring, [0, 1], [0.8, 1]);
    const opacity = interpolate(entrySpring, [0, 0.5], [0, 1], { extrapolateRight: 'clamp' });
    const exitOpacity = interpolate(frame, [durationInFrames - 15, durationInFrames], [1, 0], { extrapolateRight: 'clamp' });
    if (!content.mediaPath) return <AbsoluteFill style={{ justifyContent: 'center', alignItems: 'center' }}><GradientBackground /><div style={{ fontSize: 32, color: '#a1a1aa', fontFamily: 'Inter, system-ui, sans-serif' }}>Media not found</div></AbsoluteFill>;
    const renderMedia = () => content.mediaType === 'video' ? <OffthreadVideo src={content.mediaPath!} style={{ objectFit: 'cover', width: '100%', height: '100%' }} startFrom={content.videoStartFrom} endAt={content.videoEndAt} volume={content.videoMuted ? 0 : (content.videoVolume ?? 1)} playbackRate={content.videoPlaybackRate ?? 1} loop={content.videoLoop} /> : <Img src={content.mediaPath!} style={{ objectFit: 'cover', width: '100%', height: '100%' }} />;
    const isFullscreen = content.mediaStyle === 'fullscreen' || content.mediaStyle === 'background';
    return (
        <AbsoluteFill style={{ justifyContent: 'center', alignItems: 'center', backgroundColor: content.backgroundColor || 'transparent', overflow: 'hidden' }}>
            {!isFullscreen && <GradientBackground color1="#0a0a0a" color2={accentColor + '22'} color3="#0a0a0a" />}
            {content.title && !isFullscreen && <div style={{ position: 'absolute', top: 60, zIndex: 20, opacity: opacity * exitOpacity }}><AnimatedText text={content.title} fontSize={48} color="#ffffff" style="bounce" /></div>}
            <div style={{ transform: isFullscreen ? 'none' : `scale(${scale})`, opacity: opacity * exitOpacity, zIndex: 10, maxWidth: isFullscreen ? '100%' : '80%', maxHeight: isFullscreen ? '100%' : '70%', borderRadius: isFullscreen ? 0 : 20, overflow: 'hidden', boxShadow: isFullscreen ? 'none' : `0 20px 80px rgba(0,0,0,0.6)`, width: isFullscreen ? '100%' : undefined, height: isFullscreen ? '100%' : undefined }}>{renderMedia()}</div>
        </AbsoluteFill>
    );
};

// ─── Chart Scene ────────────────────────────────────────────
export const ChartScene: React.FC<{ content: Scene['content'] }> = ({ content }) => {
    const frame = useCurrentFrame();
    const accentColor = content.color || '#f97316';
    const chartType = content.chartType || 'bar';
    const chartData = content.chartData || content.items?.map(item => ({ label: item.label, value: item.value || 0, color: item.color })) || [];
    const titleOpacity = interpolate(frame, [0, 20], [0, 1], { extrapolateRight: 'clamp' });
    return (
        <AbsoluteFill style={{ justifyContent: 'center', alignItems: 'center', backgroundColor: content.backgroundColor || 'transparent', overflow: 'hidden', padding: 60 }}>
            <GradientBackground color1="#0a0a0a" color2={accentColor + '22'} color3="#0f0f23" />
            {content.title && <div style={{ position: 'absolute', top: 80, opacity: titleOpacity }}><AnimatedText text={content.title} fontSize={48} color="#ffffff" style="bounce" /></div>}
            <div style={{ marginTop: content.title ? 60 : 0 }}>
                {chartType === 'bar' && <BarChart data={chartData} maxValue={content.maxValue} delay={15} />}
                {chartType === 'pie' && <PieChart data={chartData} size={280} delay={15} />}
                {chartType === 'progress' && <div style={{ width: 600 }}>{chartData.map((item, i) => <ProgressBarChart key={i} value={item.value} maxValue={content.maxValue || 100} label={item.label} color={item.color || accentColor} delay={15 + i * 10} />)}</div>}
            </div>
        </AbsoluteFill>
    );
};

// ─── Comparison Scene ───────────────────────────────────────
export const ComparisonScene: React.FC<{ content: Scene['content'] }> = ({ content }) => {
    const frame = useCurrentFrame();
    const { fps } = useVideoConfig();
    const accentColor = content.color || '#f97316';
    const revealProgress = spring({ frame: frame - 20, fps, config: { damping: 15, stiffness: 80 } });
    return (
        <AbsoluteFill style={{ justifyContent: 'center', alignItems: 'center', backgroundColor: content.backgroundColor || 'transparent', overflow: 'hidden' }}>
            <GradientBackground color1="#0a0a0a" color2="#1a1a2e" color3={accentColor + '22'} />
            {content.title && <div style={{ position: 'absolute', top: 60 }}><AnimatedText text={content.title} fontSize={48} color="#ffffff" style="bounce" /></div>}
            <div style={{ display: 'flex', gap: 60, marginTop: content.title ? 40 : 0 }}>
                <div style={{ width: 400, padding: 40, backgroundColor: 'rgba(239, 68, 68, 0.15)', borderRadius: 24, border: '2px solid rgba(239, 68, 68, 0.3)', transform: `translateX(${interpolate(revealProgress, [0, 1], [-100, 0])}px)`, opacity: revealProgress }}>
                    <div style={{ fontSize: 20, color: '#ef4444', fontWeight: 600, marginBottom: 16, fontFamily: 'Inter, system-ui, sans-serif' }}>{content.beforeLabel || 'BEFORE'}</div>
                    <div style={{ fontSize: 48, color: '#ffffff', fontWeight: 800, fontFamily: 'Inter, system-ui, sans-serif' }}>{content.beforeValue || '—'}</div>
                </div>
                <div style={{ display: 'flex', alignItems: 'center', opacity: interpolate(frame, [30, 50], [0, 1], { extrapolateRight: 'clamp' }) }}><div style={{ fontSize: 60, color: accentColor }}>→</div></div>
                <div style={{ width: 400, padding: 40, backgroundColor: 'rgba(34, 197, 94, 0.15)', borderRadius: 24, border: '2px solid rgba(34, 197, 94, 0.3)', transform: `translateX(${interpolate(revealProgress, [0, 1], [100, 0])}px)`, opacity: revealProgress }}>
                    <div style={{ fontSize: 20, color: '#22c55e', fontWeight: 600, marginBottom: 16, fontFamily: 'Inter, system-ui, sans-serif' }}>{content.afterLabel || 'AFTER'}</div>
                    <div style={{ fontSize: 48, color: '#ffffff', fontWeight: 800, fontFamily: 'Inter, system-ui, sans-serif' }}>{content.afterValue || '—'}</div>
                </div>
            </div>
        </AbsoluteFill>
    );
};

// ─── Countdown Scene ────────────────────────────────────────
export const CountdownScene: React.FC<{ content: Scene['content'] }> = ({ content }) => {
    const frame = useCurrentFrame();
    const { fps, durationInFrames } = useVideoConfig();
    const accentColor = content.color || '#f97316';
    const countFrom = content.countFrom ?? 3;
    const countTo = content.countTo ?? 0;
    const totalCount = Math.abs(countFrom - countTo) + 1;
    const framesPerCount = Math.floor(durationInFrames / totalCount);
    const currentCountIndex = Math.min(Math.floor(frame / framesPerCount), totalCount - 1);
    const currentNumber = countFrom > countTo ? countFrom - currentCountIndex : countFrom + currentCountIndex;
    const frameInCount = frame % framesPerCount;
    const countProgress = spring({ frame: frameInCount, fps, config: { damping: 8, stiffness: 200 } });
    const scale = interpolate(countProgress, [0, 0.5, 1], [0.5, 1.2, 1]);
    const opacity = interpolate(frameInCount, [0, 5, framesPerCount - 5, framesPerCount], [0, 1, 1, 0], { extrapolateRight: 'clamp' });
    return (
        <AbsoluteFill style={{ justifyContent: 'center', alignItems: 'center', backgroundColor: content.backgroundColor || 'transparent', overflow: 'hidden' }}>
            <GradientBackground color1="#0a0a0a" color2={accentColor + '33'} color3="#0a0a0a" />
            <div style={{ fontSize: 300, fontWeight: 900, color: accentColor, fontFamily: 'Inter, system-ui, sans-serif', transform: `scale(${scale})`, opacity, textShadow: `0 0 60px ${accentColor}, 0 0 120px ${accentColor}66` }}>{currentNumber}</div>
        </AbsoluteFill>
    );
};

// ─── Shapes Scene ───────────────────────────────────────────
const AnimatedShape: React.FC<{ shape: any; index: number }> = ({ shape, index }) => {
    const frame = useCurrentFrame();
    const { fps } = useVideoConfig();
    const delay = shape.delay ?? index * 8;
    const entryProgress = spring({ frame: frame - delay, fps, config: { damping: 12, stiffness: 100 } });
    let scale = (shape.scale ?? 1) * interpolate(entryProgress, [0, 1], [0, 1]);
    const opacity = interpolate(entryProgress, [0, 0.5], [0, 1], { extrapolateRight: 'clamp' });
    const commonProps = { fill: shape.fill || '#f97316', stroke: shape.stroke, strokeWidth: shape.strokeWidth };
    const renderShape = () => {
        switch (shape.type) {
            case 'circle': return <Circle radius={shape.radius || 50} {...commonProps} />;
            case 'rect': return <Rect width={shape.width || 100} height={shape.height || 100} cornerRadius={shape.cornerRadius} {...commonProps} />;
            case 'triangle': return <Triangle length={shape.length || 100} direction={shape.direction || 'up'} {...commonProps} />;
            case 'star': return <Star points={shape.points || 5} innerRadius={shape.innerRadius || 30} outerRadius={shape.outerRadius || 60} {...commonProps} />;
            case 'polygon': return <Polygon points={shape.points || 6} radius={shape.radius || 50} {...commonProps} />;
            case 'ellipse': return <Ellipse rx={shape.rx || 60} ry={shape.ry || 40} {...commonProps} />;
            default: return <Circle radius={50} {...commonProps} />;
        }
    };
    return <div style={{ position: 'absolute', left: `${shape.x ?? 50}%`, top: `${shape.y ?? 50}%`, transform: `translate(-50%, -50%) scale(${scale})`, opacity, filter: shape.fill ? `drop-shadow(0 0 20px ${shape.fill}40)` : undefined }}>{renderShape()}</div>;
};

export const ShapesScene: React.FC<{ content: Scene['content'] }> = ({ content }) => {
    const accentColor = content.color || '#f97316';
    const shapes = content.shapes || [
        { type: 'circle', fill: accentColor, radius: 60, x: 30, y: 40, animation: 'pop' },
        { type: 'triangle', fill: '#3b82f6', length: 80, x: 70, y: 35, animation: 'spin', delay: 10 },
        { type: 'star', fill: '#22c55e', points: 5, innerRadius: 25, outerRadius: 50, x: 50, y: 65, animation: 'bounce', delay: 20 },
    ];
    return (
        <AbsoluteFill style={{ justifyContent: 'center', alignItems: 'center', backgroundColor: content.backgroundColor || 'transparent', overflow: 'hidden' }}>
            <GradientBackground color1="#0a0a0a" color2={accentColor + '22'} color3="#0f0f23" />
            {content.title && <div style={{ position: 'absolute', top: 80, zIndex: 20 }}><AnimatedText text={content.title} fontSize={52} color="#ffffff" style="bounce" /></div>}
            <div style={{ position: 'relative', width: '100%', height: '100%' }}>{shapes.map((shape, i) => <AnimatedShape key={i} shape={shape} index={i} />)}</div>
        </AbsoluteFill>
    );
};

// ─── Emoji Scene ────────────────────────────────────────────
export const EmojiScene: React.FC<{ content: Scene['content'] }> = ({ content }) => {
    const accentColor = content.color || '#f97316';
    const emojis = content.emojis || [{ emoji: '🔥', x: 30, y: 40, animation: 'bounce', scale: 0.2 }, { emoji: '⭐', x: 50, y: 50, animation: 'spin', scale: 0.25, delay: 10 }, { emoji: '🚀', x: 70, y: 40, animation: 'float', scale: 0.2, delay: 20 }];
    return (
        <AbsoluteFill style={{ justifyContent: 'center', alignItems: 'center', backgroundColor: content.backgroundColor || 'transparent', overflow: 'hidden' }}>
            <GradientBackground color1="#0a0a0a" color2={accentColor + '22'} color3="#0f0f23" />
            {content.title && <div style={{ position: 'absolute', top: 80, zIndex: 20 }}><AnimatedText text={content.title} fontSize={52} color="#ffffff" style="bounce" /></div>}
            <div style={{ position: 'relative', width: '100%', height: '100%' }}>
                {emojis.map((emoji, i) => {
                    const frame = useCurrentFrame();
                    const { fps } = useVideoConfig();
                    const delay = emoji.delay ?? i * 10;
                    const entry = spring({ frame: frame - delay, fps, config: { damping: 12, stiffness: 100 } });
                    const scale = (emoji.scale ?? 0.15) * interpolate(entry, [0, 1], [0, 1]);
                    return <div key={i} style={{ position: 'absolute', left: `${emoji.x ?? 50}%`, top: `${emoji.y ?? 50}%`, transform: `translate(-50%, -50%) scale(${scale})`, opacity: entry, fontSize: 120 }}>{emoji.emoji}</div>;
                })}
            </div>
        </AbsoluteFill>
    );
};

// ─── GIF Scene ──────────────────────────────────────────────
export const GifScene: React.FC<{ content: Scene['content'] }> = ({ content }) => {
    const accentColor = content.color || '#f97316';
    const gifs = content.gifs || [];
    return (
        <AbsoluteFill style={{ justifyContent: 'center', alignItems: 'center', backgroundColor: content.backgroundColor || '#0a0a0a', overflow: 'hidden' }}>
            <GradientBackground color1="#0a0a0a" color2={accentColor + '22'} color3="#0f0f23" />
            {content.title && <div style={{ position: 'absolute', top: 80, zIndex: 20 }}><AnimatedText text={content.title} fontSize={52} color="#ffffff" style="bounce" /></div>}
            <div style={{ position: 'relative', width: '100%', height: '100%' }}>
                {gifs.map((gif, i) => {
                    const frame = useCurrentFrame();
                    const { fps } = useVideoConfig();
                    const entry = spring({ frame: frame - (gif.delay || i * 5), fps, config: { damping: 12, stiffness: 150 } });
                    return <div key={i} style={{ position: 'absolute', left: `${gif.x ?? 50}%`, top: `${gif.y ?? 50}%`, transform: `translate(-50%, -50%) scale(${(gif.scale ?? 1) * entry})`, opacity: entry }}>
                        <Gif src={gif.src} width={gif.width || 300} height={gif.height} fit={gif.fit || 'contain'} playbackRate={gif.playbackRate || 1} loopBehavior={gif.loop !== false ? 'loop' : 'pause-after-finish'} style={{ borderRadius: 8 }} />
                    </div>;
                })}
            </div>
        </AbsoluteFill>
    );
};

// ─── Lottie Scene ───────────────────────────────────────────
export const LottieScene: React.FC<{ content: Scene['content'] }> = ({ content }) => {
    const accentColor = content.color || '#f97316';
    const lotties = content.lotties || [];
    return (
        <AbsoluteFill style={{ justifyContent: 'center', alignItems: 'center', backgroundColor: content.backgroundColor || '#0a0a0a', overflow: 'hidden' }}>
            <GradientBackground color1="#0a0a0a" color2={accentColor + '22'} color3="#0f0f23" />
            {content.title && <div style={{ position: 'absolute', top: 80, zIndex: 20 }}><AnimatedText text={content.title} fontSize={52} color="#ffffff" style="bounce" /></div>}
            <div style={{ position: 'relative', width: '100%', height: '100%' }}>
                {lotties.map((lottie, i) => {
                    const frame = useCurrentFrame();
                    const { fps } = useVideoConfig();
                    const [animationData, setAnimationData] = useState<LottieAnimationData | null>(null);
                    const [handle] = useState(() => delayRender('Loading Lottie'));
                    useEffect(() => { fetch(lottie.src).then(r => r.json()).then(d => { setAnimationData(d); continueRender(handle); }).catch(() => continueRender(handle)); }, [lottie.src, handle]);
                    const entry = spring({ frame: frame - (lottie.delay || i * 5), fps, config: { damping: 12, stiffness: 150 } });
                    if (!animationData || entry <= 0) return null;
                    return <div key={i} style={{ position: 'absolute', left: `${lottie.x ?? 50}%`, top: `${lottie.y ?? 50}%`, transform: `translate(-50%, -50%) scale(${(lottie.scale ?? 1) * entry})`, opacity: entry, width: lottie.width || 300, height: lottie.height || 300 }}>
                        <Lottie animationData={animationData} loop={lottie.loop} playbackRate={lottie.playbackRate} direction={lottie.direction} style={{ width: '100%', height: '100%' }} />
                    </div>;
                })}
            </div>
        </AbsoluteFill>
    );
};

// ─── Scene Renderer ─────────────────────────────────────────
export const SceneRenderer: React.FC<{ scene: Scene }> = ({ scene }) => {
    const renderScene = () => {
        switch (scene.type) {
            case 'title': return <TitleScene content={scene.content} />;
            case 'steps': case 'features': return <StepsScene content={scene.content} />;
            case 'stats': return <StatsScene content={scene.content} />;
            case 'text': return <TextScene content={scene.content} />;
            case 'transition': return <TransitionScene content={scene.content} />;
            case 'media': return <MediaScene content={scene.content} />;
            case 'chart': return <ChartScene content={scene.content} />;
            case 'comparison': return <ComparisonScene content={scene.content} />;
            case 'countdown': return <CountdownScene content={scene.content} />;
            case 'shapes': return <ShapesScene content={scene.content} />;
            case 'emoji': return <EmojiScene content={scene.content} />;
            case 'gif': return <GifScene content={scene.content} />;
            case 'lottie': return <LottieScene content={scene.content} />;
            default: return <TitleScene content={scene.content} />;
        }
    };
    let content = renderScene();
    if (scene.content.camera?.type) content = <CameraWrapper type={scene.content.camera.type} intensity={scene.content.camera.intensity}>{content}</CameraWrapper>;
    if (scene.transition?.type && scene.transition.type !== 'none') content = <TransitionWrapper transitionType={scene.transition.type} transitionDuration={scene.transition.duration || 15}>{content}</TransitionWrapper>;
    return content;
};
