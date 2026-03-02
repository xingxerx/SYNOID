import React, { useCallback, useRef, useState, useEffect } from 'react';
import type { Track, TimelineClip, Asset, CaptionData } from '../types';

const TRACK_COLORS: Record<string, string> = {
    text: 'rgba(180,100,255,0.85)',
    video: 'rgba(74,158,255,0.85)',
    audio: 'rgba(61,190,122,0.85)',
};

interface Props {
    tracks: Track[];
    clips: TimelineClip[];
    assets: Asset[];
    captionData: Record<string, CaptionData>;
    duration: number;
    playheadPosition: number;
    timelineZoom: number;          // px per second
    isPlaying: boolean;
    selectedClipId: string | null;
    onSeek: (t: number) => void;
    onSelectClip: (id: string | null) => void;
    onMoveClip: (id: string, start: number, trackId?: string) => void;
    onSplitClip: (id: string, time: number) => void;
    onDeleteClip: (id: string) => void;
    onZoomIn: () => void;
    onZoomOut: () => void;
    onPlayPause: () => void;
    onZoomChange: (z: number) => void;
}

function formatRulerTime(s: number): string {
    const m = Math.floor(s / 60);
    const sec = Math.floor(s % 60);
    return `${m}:${String(sec).padStart(2, '0')}`;
}

export function Timeline({
    tracks, clips, assets, captionData, duration, playheadPosition,
    timelineZoom, isPlaying, selectedClipId,
    onSeek, onSelectClip, onMoveClip, onSplitClip, onDeleteClip,
    onZoomIn, onZoomOut, onPlayPause, onZoomChange,
}: Props) {
    const scrollRef = useRef<HTMLDivElement>(null);
    const dragging = useRef<{ clipId: string; offsetX: number } | null>(null);

    const totalWidth = Math.max(duration * timelineZoom + 300, 800);

    // Auto-scroll to keep playhead visible
    useEffect(() => {
        const el = scrollRef.current;
        if (!el) return;
        const headX = playheadPosition * timelineZoom;
        const { scrollLeft, clientWidth } = el;
        if (headX < scrollLeft + 40 || headX > scrollLeft + clientWidth - 40) {
            el.scrollLeft = Math.max(0, headX - clientWidth * 0.35);
        }
    }, [playheadPosition, timelineZoom]);

    const handleRulerClick = useCallback((e: React.MouseEvent<HTMLDivElement>) => {
        const el = scrollRef.current;
        if (!el) return;
        const rect = (e.currentTarget as HTMLDivElement).getBoundingClientRect();
        const x = e.clientX - rect.left + el.scrollLeft;
        onSeek(x / timelineZoom);
    }, [timelineZoom, onSeek]);

    const handleClipMouseDown = useCallback((e: React.MouseEvent, clipId: string) => {
        e.stopPropagation();
        onSelectClip(clipId);
        const clip = clips.find(c => c.id === clipId);
        if (!clip) return;
        const clipStartX = clip.start * timelineZoom;
        const offsetX = e.clientX - clipStartX;
        dragging.current = { clipId, offsetX };

        const onMove = (me: MouseEvent) => {
            if (!dragging.current) return;
            const el = scrollRef.current;
            const scrollX = el?.scrollLeft ?? 0;
            const newX = me.clientX - dragging.current.offsetX + scrollX;
            const newStart = Math.max(0, newX / timelineZoom);
            onMoveClip(dragging.current.clipId, newStart);
        };
        const onUp = () => {
            dragging.current = null;
            window.removeEventListener('mousemove', onMove);
            window.removeEventListener('mouseup', onUp);
        };
        window.addEventListener('mousemove', onMove);
        window.addEventListener('mouseup', onUp);
    }, [clips, timelineZoom, onSelectClip, onMoveClip]);

    const handleTrackClick = useCallback((e: React.MouseEvent, trackId: string) => {
        if (dragging.current) return;
        onSelectClip(null);
    }, [onSelectClip]);

    // Ruler tick interval
    const getTickInterval = () => {
        if (timelineZoom >= 100) return 1;
        if (timelineZoom >= 40) return 5;
        if (timelineZoom >= 15) return 10;
        if (timelineZoom >= 5) return 30;
        return 60;
    };

    const tickInterval = getTickInterval();
    const numTicks = Math.ceil(totalWidth / timelineZoom / tickInterval) + 1;

    const rulerTicks = Array.from({ length: numTicks }, (_, i) => ({
        time: i * tickInterval,
        x: i * tickInterval * timelineZoom,
        label: formatRulerTime(i * tickInterval),
    }));

    return (
        <div className="timeline">
            {/* Timeline toolbar */}
            <div className="timeline-toolbar">
                <button className="tl-btn" onClick={() => onSeek(0)} title="Go to start">⏮</button>
                <button className={`tl-btn${isPlaying ? ' active' : ''}`} onClick={onPlayPause} title="Play/Pause">
                    {isPlaying ? '⏸' : '▶'}
                </button>
                <button className="tl-btn" onClick={() => onSeek(duration)} title="Go to end">⏭</button>
                <button className="tl-btn" title="Split at playhead (S)" onClick={() => {
                    if (selectedClipId) onSplitClip(selectedClipId, playheadPosition);
                }}>✂</button>
                <button className="tl-btn" title="Delete selected" onClick={() => {
                    if (selectedClipId) onDeleteClip(selectedClipId);
                }}>🗑</button>

                <div className="tl-spacer" />

                <div className="zoom-ctrl">
                    <span>−</span>
                    <input
                        type="range"
                        min={5} max={200} step={1}
                        value={timelineZoom}
                        onChange={e => onZoomChange(Number(e.target.value))}
                        title="Timeline zoom"
                    />
                    <span>+</span>
                </div>
            </div>

            {/* Body: track labels + scroll body */}
            <div className="timeline-body">
                {/* Track labels */}
                <div className="track-labels">
                    <div className="track-label" style={{ height: 20, background: 'var(--bg-base)', fontSize: 9, color: 'var(--text-dim)' }}>TRACKS</div>
                    {tracks.map(t => (
                        <div key={t.id} className="track-label">
                            <span style={{ marginRight: 4, fontSize: 10 }}>
                                {t.type === 'text' ? '💬' : t.type === 'video' ? '▶' : '🎵'}
                            </span>
                            {t.name}
                        </div>
                    ))}
                </div>

                {/* Scrollable track area */}
                <div className="timeline-scroll" ref={scrollRef}>
                    <div className="timeline-canvas-area" style={{ width: totalWidth }}>
                        {/* Ruler */}
                        <div className="timeline-ruler" style={{ width: totalWidth }} onClick={handleRulerClick}>
                            {rulerTicks.map(tick => (
                                <div key={tick.time} className="ruler-tick" style={{ left: tick.x }}>
                                    <div style={{ position: 'absolute', height: 8, width: 1, background: 'var(--border)', bottom: 0, left: 0 }} />
                                    {tick.time % (tickInterval * 2) === 0 && (
                                        <span style={{ position: 'absolute', bottom: 8, left: 3 }}>{tick.label}</span>
                                    )}
                                </div>
                            ))}
                        </div>

                        {/* Track rows */}
                        <div className="tracks-area" style={{ width: totalWidth }}>
                            {tracks.map(track => {
                                const trackClips = clips.filter(c => c.trackId === track.id);
                                return (
                                    <div
                                        key={track.id}
                                        className="track-row"
                                        style={{ width: totalWidth }}
                                        onClick={(e) => handleTrackClick(e, track.id)}
                                    >
                                        {trackClips.map(clip => {
                                            const asset = assets.find(a => a.id === clip.assetId);
                                            const data = captionData[clip.id];
                                            const label = data ? '💬 Captions' : (asset?.filename ?? 'Clip');
                                            const color = TRACK_COLORS[track.type] ?? 'rgba(100,100,100,0.8)';
                                            const w = Math.max(clip.duration * timelineZoom, 8);

                                            return (
                                                <div
                                                    key={clip.id}
                                                    className={`clip-block${selectedClipId === clip.id ? ' selected' : ''}`}
                                                    style={{
                                                        left: clip.start * timelineZoom,
                                                        width: w,
                                                        background: color,
                                                    }}
                                                    onMouseDown={e => handleClipMouseDown(e, clip.id)}
                                                    title={`${label} — ${clip.duration.toFixed(1)}s`}
                                                >
                                                    {w > 40 ? label : ''}
                                                </div>
                                            );
                                        })}
                                    </div>
                                );
                            })}
                        </div>

                        {/* Playhead */}
                        <div
                            className="playhead"
                            style={{ left: playheadPosition * timelineZoom, top: 0, bottom: 0 }}
                        />
                    </div>
                </div>
            </div>
        </div>
    );
}
