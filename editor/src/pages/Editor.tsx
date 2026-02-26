import React, { useCallback, useEffect, useMemo, useState } from 'react';
import { useProject } from '../hooks/useProject';
import { MediaBin } from '../components/MediaBin';
import { PreviewPanel } from '../components/PreviewPanel';
import { Timeline } from '../components/Timeline';
import { DirectorPanel } from '../components/DirectorPanel';
import { PropertiesPanel } from '../components/PropertiesPanel';
import { RenderModal } from '../components/RenderModal';
import type { AIAction } from '../types';
import * as api from '../api';

type RightTab = 'director' | 'properties' | 'captions';

export function Editor() {
    const p = useProject();
    const [rightTab, setRightTab] = useState<RightTab>('director');
    const [selectedAssetId, setSelectedAssetId] = useState<string | null>(null);
    const [showRender, setShowRender] = useState(false);
    const [uploading, setUploading] = useState(false);
    const [editRunning, setEditRunning] = useState(false);

    // Derive the preview video URL from the main Video clip
    const previewVideoSrc = useMemo(() => {
        // Find the main video clip on V1
        const mainClip = p.clips.find(c => c.trackId === 'V1');
        if (!mainClip) return undefined;
        const asset = p.assets.find(a => a.id === mainClip.assetId);
        return asset?.streamUrl;
    }, [p.clips, p.assets]);

    // Handle file upload
    const handleUpload = useCallback(async (file: File) => {
        setUploading(true);
        try {
            const asset = await p.uploadAsset(file);
            if (asset) {
                setSelectedAssetId(asset.id);
                // Add to timeline automatically (main video on V1, audio on A1)
                const trackId = asset.type === 'audio' ? 'A1' : 'V1';
                const start = p.clips.filter(c => c.trackId === trackId)
                    .reduce((max, c) => Math.max(max, c.start + c.duration), 0);
                p.addClip(asset.id, trackId, start);
            }
        } finally {
            setUploading(false);
        }
    }, [p, p.assets, p.clips]);

    // AI Director action handler
    const handleAIAction = useCallback(async (action: AIAction) => {
        if (!p.sessionId) return;

        if (action.type === 'transcribe') {
            const mainAsset = p.assets.find(a => a.type === 'video');
            if (mainAsset) {
                await p.transcribeAsset(mainAsset.id);
                setRightTab('captions');
            }
        } else if (action.type === 'auto-edit') {
            const intent = action.params.intent as string;
            const mainAsset = p.assets.find(a => a.type === 'video');
            if (!mainAsset) return;

            setEditRunning(true);
            try {
                await api.aiAutoEdit(p.sessionId, 'smart-edit', {
                    intent,
                    assetId: mainAsset.id,
                });
                // Poll for completion
                const poll = setInterval(async () => {
                    const s = await api.getRenderStatus(p.sessionId!);
                    if (s.status === 'done' || s.status === 'error') {
                        clearInterval(poll);
                        setEditRunning(false);
                    }
                }, 1500);
            } catch {
                setEditRunning(false);
            }
        }
    }, [p]);

    // Export handler
    const handleExport = useCallback(async () => {
        if (!p.sessionId) return;
        const mainAsset = p.assets.find(a => a.type === 'video');
        await api.startRender(p.sessionId, {
            assetId: mainAsset?.id,
            clips: p.clips,
            captionData: p.captionData,
        });
        setShowRender(true);
    }, [p]);

    // Transcribe shortcut
    const handleTranscribeFromProps = useCallback(async (assetId: string) => {
        await p.transcribeAsset(assetId);
        setRightTab('captions');
    }, [p]);

    const projectNameDisplay = p.assets[0]?.filename ?? 'New Project';

    return (
        <div className="editor-root">
            {/* ── Toolbar ── */}
            <header className="editor-toolbar">
                <span className="toolbar-logo" title="SYNOID">▶ SYNOID</span>
                <div className="toolbar-divider" />

                {/* Undo/Redo */}
                <button className="toolbar-btn" title="Undo (Ctrl+Z)">↶</button>
                <button className="toolbar-btn" title="Redo (Ctrl+Y)">↷</button>

                <div className="toolbar-divider" />

                {/* Project name */}
                <span className="toolbar-project-name">{projectNameDisplay}</span>

                <div className="toolbar-spacer" />

                {/* Playback controls */}
                <button className="toolbar-btn" onClick={p.skipBack} title="Skip back 5s">⏪</button>
                <button className="toolbar-play-btn" onClick={p.playPause} title="Play / Pause (Space)">
                    {p.isPlaying ? '⏸' : '▶'}
                </button>
                <button className="toolbar-btn" onClick={p.skipForward} title="Skip forward 5s">⏩</button>

                <div className="toolbar-divider" />

                {/* Timecode */}
                <div className="toolbar-timecode">{p.playheadTime}</div>
                <span style={{ fontSize: 11, color: 'var(--text-dim)' }}>/</span>
                <div className="toolbar-timecode" style={{ color: 'var(--text-secondary)' }}>{p.durationTime}</div>

                <div className="toolbar-spacer" />

                {/* Export */}
                <button
                    className="toolbar-export"
                    onClick={handleExport}
                    disabled={p.assets.length === 0}
                    title="Export video"
                >
                    🎬 Export
                </button>
            </header>

            {/* ── Media Bin ── */}
            <MediaBin
                assets={p.assets}
                isUploading={uploading}
                onUpload={handleUpload}
                onAddClip={(assetId, trackId, start) => p.addClip(assetId, trackId, start)}
                selectedAssetId={selectedAssetId}
                onSelectAsset={setSelectedAssetId}
                onDeleteAsset={p.deleteAsset}
            />

            {/* ── Preview ── */}
            <PreviewPanel
                videoSrc={previewVideoSrc}
                isPlaying={p.isPlaying}
                playheadPosition={p.playheadPosition}
                duration={p.duration}
                clips={p.clips}
                captionData={p.captionData}
                onTimeUpdate={p.setPlayheadPosition}
                onPlayPause={p.playPause}
                onDurationChange={(d) => { /* duration tracked from clips */ }}
            />

            {/* ── Right Panel ── */}
            <aside className="right-panel">
                <div className="right-panel-tabs">
                    {(['director', 'properties', 'captions'] as RightTab[]).map(tab => (
                        <button
                            key={tab}
                            className={`right-panel-tab${rightTab === tab ? ' active' : ''}`}
                            onClick={() => setRightTab(tab)}
                        >
                            {tab === 'director' ? '🎬 Director' : tab === 'properties' ? '⚙ Props' : '💬 Captions'}
                        </button>
                    ))}
                </div>

                <div className="right-panel-body">
                    {rightTab === 'director' && (
                        <DirectorPanel
                            sessionId={p.sessionId}
                            assetId={p.assets.find(a => a.type === 'video')?.id}
                            onAction={handleAIAction}
                        />
                    )}
                    {rightTab === 'properties' && (
                        <PropertiesPanel
                            selectedClip={p.selectedClip}
                            captionData={p.captionData}
                            onUpdateCaptionStyle={p.updateCaptionStyle}
                            onUpdateClip={p.updateClip}
                            isTranscribing={p.isTranscribing}
                            onTranscribe={handleTranscribeFromProps}
                        />
                    )}
                    {rightTab === 'captions' && (
                        <CaptionsView
                            sessionId={p.sessionId}
                            assets={p.assets}
                            captionData={p.captionData}
                            clips={p.clips}
                            isTranscribing={p.isTranscribing}
                            onTranscribe={() => {
                                const vid = p.assets.find(a => a.type === 'video');
                                if (vid) p.transcribeAsset(vid.id);
                            }}
                        />
                    )}
                </div>
            </aside>

            {/* ── Timeline ── */}
            <Timeline
                tracks={p.tracks}
                clips={p.clips}
                assets={p.assets}
                captionData={p.captionData}
                duration={p.duration}
                playheadPosition={p.playheadPosition}
                timelineZoom={p.timelineZoom}
                isPlaying={p.isPlaying}
                selectedClipId={p.selectedClipId}
                onSeek={p.setPlayheadPosition}
                onSelectClip={p.setSelectedClipId}
                onMoveClip={(id, start, trackId) => p.moveClip(id, start, trackId)}
                onSplitClip={p.splitClip}
                onDeleteClip={p.deleteClip}
                onZoomIn={p.zoomIn}
                onZoomOut={p.zoomOut}
                onPlayPause={p.playPause}
                onZoomChange={(z) => {
                    const delta = z - p.timelineZoom;
                    if (delta > 0) p.zoomIn();
                    else p.zoomOut();
                }}
            />

            {/* ── Render Modal ── */}
            {showRender && (
                <RenderModal
                    sessionId={p.sessionId}
                    onClose={() => setShowRender(false)}
                />
            )}

            {/* ── AI Edit running indicator ── */}
            {editRunning && (
                <div style={{
                    position: 'fixed', bottom: 24, right: 24,
                    background: 'var(--bg-elevated)', border: '1px solid var(--gold)',
                    borderRadius: 8, padding: '10px 16px',
                    display: 'flex', alignItems: 'center', gap: 10,
                    zIndex: 200, fontSize: 12,
                }}>
                    <span className="spinner" />
                    AI Edit running…
                </div>
            )}
        </div>
    );
}

// ── Captions View (inside right panel) ────────────────────────────────────────
function CaptionsView({ sessionId, assets, captionData, clips, isTranscribing, onTranscribe }: {
    sessionId: string | null;
    assets: any[];
    captionData: Record<string, any>;
    clips: any[];
    isTranscribing: boolean;
    onTranscribe: () => void;
}) {
    const captionClip = clips.find(c => c.trackId === 'T1' && captionData[c.id]);
    const data = captionClip ? captionData[captionClip.id] : null;
    const hasVideo = assets.some(a => a.type === 'video');

    return (
        <div>
            {!hasVideo && (
                <div className="captions-empty">
                    <div>🎬</div>
                    <div>Import a video first</div>
                </div>
            )}

            {hasVideo && !data && (
                <div className="captions-empty">
                    <div style={{ fontSize: 32, marginBottom: 8 }}>💬</div>
                    <div style={{ marginBottom: 4 }}>No captions yet</div>
                    <div style={{ fontSize: 11, color: 'var(--text-dim)', marginBottom: 16 }}>
                        Transcribe your audio to generate word-level captions
                    </div>
                    {isTranscribing ? (
                        <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                            <span className="spinner" />
                            <span style={{ fontSize: 11 }}>Transcribing… this takes a while</span>
                        </div>
                    ) : (
                        <button className="captions-transcribe-btn" onClick={onTranscribe}>
                            🎙 Transcribe Audio
                        </button>
                    )}
                </div>
            )}

            {data && (
                <div>
                    <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 10 }}>
                        <span style={{ fontSize: 11, color: 'var(--accent-green)' }}>
                            ✅ {data.words.length} words
                        </span>
                        <button
                            style={{ fontSize: 11, padding: '3px 8px', background: 'var(--bg-input)', color: 'var(--text-secondary)', borderRadius: 4 }}
                            onClick={onTranscribe}
                            disabled={isTranscribing}
                        >
                            {isTranscribing ? '...' : 'Re-transcribe'}
                        </button>
                    </div>
                    <div style={{ maxHeight: 300, overflowY: 'auto', fontSize: 11, color: 'var(--text-secondary)', lineHeight: 1.7 }}>
                        {data.words.map((w: any, i: number) => (
                            <span key={i} title={`${w.start.toFixed(2)}s – ${w.end.toFixed(2)}s`}>
                                {w.text}{' '}
                            </span>
                        ))}
                    </div>
                </div>
            )}
        </div>
    );
}
