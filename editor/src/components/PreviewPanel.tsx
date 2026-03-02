import React, { useCallback, useEffect, useRef } from 'react';
import type { CaptionData, TimelineClip } from '../types';

interface Props {
    videoSrc?: string;           // URL of the video to play
    isPlaying: boolean;
    playheadPosition: number;
    duration: number;
    clips: TimelineClip[];
    captionData: Record<string, CaptionData>;
    onTimeUpdate: (t: number) => void;
    onPlayPause: () => void;
    onDurationChange: (d: number) => void;
}

export function PreviewPanel({
    videoSrc, isPlaying, playheadPosition, duration,
    clips, captionData, onTimeUpdate, onPlayPause, onDurationChange,
}: Props) {
    const videoRef = useRef<HTMLVideoElement>(null);
    const canvasRef = useRef<HTMLCanvasElement>(null);
    const seekingRef = useRef(false);

    // Sync play/pause with external state
    useEffect(() => {
        const vid = videoRef.current;
        if (!vid) return;
        if (isPlaying) {
            vid.play().catch(() => { });
        } else {
            vid.pause();
        }
    }, [isPlaying]);

    // Sync seek when playhead changes externally (but not from video timeUpdate)
    useEffect(() => {
        const vid = videoRef.current;
        if (!vid || seekingRef.current) return;
        if (Math.abs(vid.currentTime - playheadPosition) > 0.25) {
            vid.currentTime = playheadPosition;
        }
    }, [playheadPosition]);

    const handleTimeUpdate = useCallback(() => {
        const vid = videoRef.current;
        if (!vid) return;
        seekingRef.current = false;
        onTimeUpdate(vid.currentTime);
        drawSubtitles(vid.currentTime);
    }, [onTimeUpdate]);

    const handleDurationChange = useCallback(() => {
        const vid = videoRef.current;
        if (vid) onDurationChange(vid.duration);
    }, [onDurationChange]);

    // Draw subtitle overlay
    const drawSubtitles = useCallback((currentTime: number) => {
        const canvas = canvasRef.current;
        const vid = videoRef.current;
        if (!canvas || !vid) return;

        canvas.width = vid.clientWidth;
        canvas.height = vid.clientHeight;

        const ctx = canvas.getContext('2d');
        if (!ctx) return;
        ctx.clearRect(0, 0, canvas.width, canvas.height);

        // Find active caption clips
        const captionClip = clips.find(c => c.trackId === 'T1' && captionData[c.id]);
        if (!captionClip) return;

        const data = captionData[captionClip.id];
        if (!data?.words?.length) return;

        const style = data.style;
        const fontSize = Math.max(12, style.fontSize * (canvas.height / 1080));
        const fontWeight = style.fontWeight === 'black' ? '900' : style.fontWeight;
        ctx.font = `${fontWeight} ${fontSize}px ${style.fontFamily}`;
        ctx.textAlign = 'center';

        // Get words for current time (relative to clip start)
        const relTime = currentTime - captionClip.start;

        // Find the sentence group visible at this time
        const activeWords = data.words.filter(w => relTime >= w.start && relTime < w.end + 0.5);
        // Show all words in the sentence that contains the active word
        const allCurrentWords = data.words.filter(w => {
            // words within 3s window of relTime
            return w.end >= relTime - 0.05 && w.start <= relTime + 3;
        }).slice(0, 12);

        if (!allCurrentWords.length) return;

        const text = allCurrentWords.map(w => w.text).join(' ');
        const posY = style.position === 'top' ? canvas.height * 0.12 :
            style.position === 'center' ? canvas.height * 0.5 :
                canvas.height * 0.85;

        const padding = 12;
        const textWidth = ctx.measureText(text).width;
        const boxW = textWidth + padding * 2;
        const boxH = fontSize + padding;
        const boxX = canvas.width / 2 - boxW / 2;
        const boxY = posY - boxH / 2;

        // Background
        if (style.backgroundColor !== 'transparent') {
            ctx.fillStyle = style.backgroundColor;
            roundRect(ctx, boxX, boxY, boxW, boxH, 4);
            ctx.fill();
        }

        // Stroke
        if (style.strokeWidth > 0) {
            ctx.strokeStyle = style.strokeColor;
            ctx.lineWidth = style.strokeWidth;
            ctx.strokeText(text, canvas.width / 2, posY + fontSize * 0.35);
        }

        // For karaoke: draw base text then highlight spoken words
        if (style.animation === 'karaoke') {
            ctx.fillStyle = style.color;
            ctx.fillText(text, canvas.width / 2, posY + fontSize * 0.35);

            // Highlight spoken words
            let xOffset = canvas.width / 2 - textWidth / 2;
            for (const word of allCurrentWords) {
                const wText = word.text + ' ';
                const wWidth = ctx.measureText(wText).width;
                if (relTime >= word.start) {
                    ctx.fillStyle = style.highlightColor;
                    ctx.fillText(wText, xOffset + wWidth / 2, posY + fontSize * 0.35);
                }
                xOffset += wWidth;
            }
        } else {
            ctx.fillStyle = style.color;
            ctx.fillText(text, canvas.width / 2, posY + fontSize * 0.35);
        }
    }, [clips, captionData]);

    // Redraw subtitles periodically when playing
    useEffect(() => {
        if (!isPlaying) {
            drawSubtitles(playheadPosition);
            return;
        }
        const id = requestAnimationFrame(function loop() {
            const vid = videoRef.current;
            if (vid) drawSubtitles(vid.currentTime);
            if (isPlaying) requestAnimationFrame(loop);
        });
        return () => cancelAnimationFrame(id);
    }, [isPlaying, drawSubtitles, playheadPosition]);

    return (
        <div className="preview-panel">
            <div className="preview-container" onClick={onPlayPause}>
                {videoSrc ? (
                    <>
                        <video
                            ref={videoRef}
                            className="preview-video"
                            src={videoSrc}
                            onTimeUpdate={handleTimeUpdate}
                            onDurationChange={handleDurationChange}
                            onEnded={() => seekingRef.current = false}
                            preload="metadata"
                            playsInline
                            style={{ cursor: 'pointer' }}
                        />
                        <canvas ref={canvasRef} className="preview-canvas" />
                    </>
                ) : (
                    <div className="preview-placeholder">
                        <div className="play-icon">▶</div>
                        <span style={{ fontSize: 12 }}>Import a video to begin</span>
                    </div>
                )}
            </div>
        </div>
    );
}

function roundRect(ctx: CanvasRenderingContext2D, x: number, y: number, w: number, h: number, r: number) {
    ctx.beginPath();
    ctx.moveTo(x + r, y);
    ctx.lineTo(x + w - r, y);
    ctx.quadraticCurveTo(x + w, y, x + w, y + r);
    ctx.lineTo(x + w, y + h - r);
    ctx.quadraticCurveTo(x + w, y + h, x + w - r, y + h);
    ctx.lineTo(x + r, y + h);
    ctx.quadraticCurveTo(x, y + h, x, y + h - r);
    ctx.lineTo(x, y + r);
    ctx.quadraticCurveTo(x, y, x + r, y);
    ctx.closePath();
}
