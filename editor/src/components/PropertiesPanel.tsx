import React from 'react';
import type { TimelineClip, CaptionData, CaptionStyle, ClipTransform } from '../types';

const CAPTION_COLORS = ['#ffffff', '#ffff00', '#ff7832', '#00ff88', '#4a9eff', '#ff4444'];
const ANIMATIONS = ['none', 'karaoke', 'fade', 'pop', 'bounce', 'typewriter'] as const;
const POSITIONS = ['top', 'center', 'bottom'] as const;
const FONTS = ['Inter', 'Arial', 'Impact', 'Georgia', 'Courier New', 'Comic Sans MS'];

interface Props {
    selectedClip: TimelineClip | null;
    captionData: Record<string, CaptionData>;
    onUpdateCaptionStyle: (clipId: string, style: Partial<CaptionStyle>) => void;
    onUpdateClip: (clipId: string, updates: Partial<TimelineClip>) => void;
    isTranscribing: boolean;
    onTranscribe: (assetId: string) => void;
}

export function PropertiesPanel({
    selectedClip, captionData, onUpdateCaptionStyle, onUpdateClip, isTranscribing, onTranscribe
}: Props) {
    const captionClip = selectedClip && captionData[selectedClip.id];
    const style = captionClip?.style;

    if (!selectedClip) {
        return (
            <div style={{ padding: '20px 12px', color: 'var(--text-dim)', fontSize: 12, textAlign: 'center' }}>
                <div style={{ fontSize: 24, marginBottom: 8 }}>☝️</div>
                Select a clip to view its properties
            </div>
        );
    }

    return (
        <div>
            {/* Clip basics */}
            <div className="prop-section">
                <h4>Clip</h4>
                <div className="prop-row">
                    <span className="prop-label">Start</span>
                    <input
                        className="prop-input"
                        type="number"
                        step={0.1}
                        value={selectedClip.start.toFixed(2)}
                        onChange={e => onUpdateClip(selectedClip.id, { start: parseFloat(e.target.value) || 0 })}
                    />
                </div>
                <div className="prop-row">
                    <span className="prop-label">Duration</span>
                    <input
                        className="prop-input"
                        type="number"
                        step={0.1}
                        min={0.1}
                        value={selectedClip.duration.toFixed(2)}
                        onChange={e => onUpdateClip(selectedClip.id, { duration: parseFloat(e.target.value) || 1 })}
                    />
                </div>
                <div className="prop-row">
                    <span className="prop-label">Speed</span>
                    <input
                        className="prop-slider"
                        type="range"
                        min={0.25} max={4} step={0.05}
                        value={selectedClip.speed}
                        onChange={e => onUpdateClip(selectedClip.id, { speed: parseFloat(e.target.value) })}
                    />
                    <span style={{ minWidth: 36, fontSize: 11, color: 'var(--text-secondary)' }}>
                        {selectedClip.speed.toFixed(2)}x
                    </span>
                </div>
                {selectedClip.trackId !== 'T1' && (
                    <div className="prop-row">
                        <span className="prop-label">Volume</span>
                        <input
                            className="prop-slider"
                            type="range"
                            min={0} max={2} step={0.01}
                            value={selectedClip.volume}
                            onChange={e => onUpdateClip(selectedClip.id, { volume: parseFloat(e.target.value) })}
                        />
                        <span style={{ minWidth: 36, fontSize: 11, color: 'var(--text-secondary)' }}>
                            {Math.round(selectedClip.volume * 100)}%
                        </span>
                    </div>
                )}
            </div>

            {/* Caption Style section */}
            {style ? (
                <div className="prop-section">
                    <h4>Caption Style</h4>

                    <div className="prop-row">
                        <span className="prop-label">Font</span>
                        <select
                            className="prop-input"
                            value={style.fontFamily}
                            onChange={e => onUpdateCaptionStyle(selectedClip.id, { fontFamily: e.target.value })}
                        >
                            {FONTS.map(f => <option key={f} value={f}>{f}</option>)}
                        </select>
                    </div>

                    <div className="prop-row">
                        <span className="prop-label">Size</span>
                        <input
                            className="caption-size-slider"
                            type="range"
                            min={20} max={100} step={2}
                            value={style.fontSize}
                            onChange={e => onUpdateCaptionStyle(selectedClip.id, { fontSize: parseInt(e.target.value) })}
                        />
                        <span style={{ minWidth: 30, fontSize: 11, color: 'var(--text-secondary)' }}>{style.fontSize}</span>
                    </div>

                    <div className="prop-row">
                        <span className="prop-label">Color</span>
                        <div className="caption-color-row">
                            {CAPTION_COLORS.map(c => (
                                <div
                                    key={c}
                                    className={`caption-color-swatch${style.color === c ? ' active' : ''}`}
                                    style={{ background: c }}
                                    onClick={() => onUpdateCaptionStyle(selectedClip.id, { color: c })}
                                />
                            ))}
                            <input
                                type="color"
                                value={style.color}
                                onChange={e => onUpdateCaptionStyle(selectedClip.id, { color: e.target.value })}
                                style={{ width: 22, height: 22, border: 'none', background: 'none', cursor: 'pointer' }}
                            />
                        </div>
                    </div>

                    <div className="prop-row">
                        <span className="prop-label">Highlight</span>
                        <div className="caption-color-row">
                            {['#ff7832', '#ffff00', '#00ff88', '#4a9eff', '#ff4444'].map(c => (
                                <div
                                    key={c}
                                    className={`caption-color-swatch${style.highlightColor === c ? ' active' : ''}`}
                                    style={{ background: c }}
                                    onClick={() => onUpdateCaptionStyle(selectedClip.id, { highlightColor: c })}
                                />
                            ))}
                        </div>
                    </div>

                    <div className="prop-row" style={{ flexWrap: 'wrap', gap: 4 }}>
                        <span className="prop-label">Animation</span>
                        <div style={{ display: 'flex', gap: 4, flexWrap: 'wrap' }}>
                            {ANIMATIONS.map(a => (
                                <button
                                    key={a}
                                    className={`caption-anim-btn${style.animation === a ? ' active' : ''}`}
                                    onClick={() => onUpdateCaptionStyle(selectedClip.id, { animation: a })}
                                >
                                    {a}
                                </button>
                            ))}
                        </div>
                    </div>

                    <div className="prop-row" style={{ flexWrap: 'wrap', gap: 4 }}>
                        <span className="prop-label">Position</span>
                        <div style={{ display: 'flex', gap: 4 }}>
                            {POSITIONS.map(p => (
                                <button
                                    key={p}
                                    className={`caption-anim-btn${style.position === p ? ' active' : ''}`}
                                    onClick={() => onUpdateCaptionStyle(selectedClip.id, { position: p })}
                                >
                                    {p}
                                </button>
                            ))}
                        </div>
                    </div>
                </div>
            ) : selectedClip.trackId === 'T1' ? (
                <div className="prop-section">
                    <h4>Captions</h4>
                    <div style={{ fontSize: 11, color: 'var(--text-secondary)', marginBottom: 8 }}>
                        Transcribe audio to generate captions with word-level timing.
                    </div>
                    {isTranscribing ? (
                        <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                            <span className="spinner" /><span style={{ fontSize: 11 }}>Transcribing…</span>
                        </div>
                    ) : (
                        <button
                            className="captions-transcribe-btn"
                            onClick={() => selectedClip.assetId && onTranscribe(selectedClip.assetId)}
                            disabled={!selectedClip.assetId}
                        >
                            🎙 Transcribe Audio
                        </button>
                    )}
                </div>
            ) : null}
        </div>
    );
}
