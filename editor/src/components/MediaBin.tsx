import React, { useCallback, useRef, useState } from 'react';
import type { Asset, TimelineClip } from '../types';

interface Props {
    assets: Asset[];
    isUploading: boolean;
    onUpload: (file: File) => void;
    onAddClip: (assetId: string, trackId: string, start: number) => void;
    selectedAssetId?: string | null;
    onSelectAsset?: (id: string) => void;
    onDeleteAsset?: (id: string) => void;
}

function formatDur(s: number): string {
    if (!s) return '—';
    const m = Math.floor(s / 60);
    const sec = Math.floor(s % 60);
    return `${m}:${String(sec).padStart(2, '0')}`;
}

export function MediaBin({
    assets, isUploading, onUpload, onAddClip,
    selectedAssetId, onSelectAsset, onDeleteAsset,
}: Props) {
    const [dragging, setDragging] = useState(false);
    const inputRef = useRef<HTMLInputElement>(null);

    const handleDrop = useCallback((e: React.DragEvent) => {
        e.preventDefault();
        setDragging(false);
        const file = e.dataTransfer.files[0];
        if (file) onUpload(file);
    }, [onUpload]);

    const handleClick = () => inputRef.current?.click();

    return (
        <aside className="media-bin">
            <div className="media-bin-header">
                <span className="panel-title">Media</span>
                {isUploading && <span className="spinner" />}
            </div>

            {/* Drop zone */}
            <div
                className={`media-upload-zone${dragging ? ' drag-over' : ''}`}
                onClick={handleClick}
                onDragOver={e => { e.preventDefault(); setDragging(true); }}
                onDragLeave={() => setDragging(false)}
                onDrop={handleDrop}
            >
                <div className="upload-icon">☁</div>
                <p>Drop video / audio here</p>
                <p style={{ marginTop: 2, fontSize: 10, color: 'var(--text-dim)' }}>or click to browse</p>
            </div>
            <input
                ref={inputRef}
                type="file"
                accept="video/*,audio/*,image/*"
                style={{ display: 'none' }}
                onChange={e => e.target.files?.[0] && onUpload(e.target.files[0])}
            />

            {/* Asset grid */}
            <div className="asset-grid">
                {assets.map(asset => (
                    <AssetCard
                        key={asset.id}
                        asset={asset}
                        selected={selectedAssetId === asset.id}
                        onSelect={() => onSelectAsset?.(asset.id)}
                        onDelete={() => onDeleteAsset?.(asset.id)}
                        onAddToTimeline={() => onAddClip(asset.id, 'V1', 0)}
                    />
                ))}
                {assets.length === 0 && !isUploading && (
                    <div style={{ gridColumn: '1/-1', textAlign: 'center', color: 'var(--text-dim)', padding: '24px 0', fontSize: 11 }}>
                        No assets yet
                    </div>
                )}
            </div>
        </aside>
    );
}

interface CardProps {
    asset: Asset;
    selected: boolean;
    onSelect: () => void;
    onDelete: () => void;
    onAddToTimeline: () => void;
}

function AssetCard({ asset, selected, onSelect, onDelete, onAddToTimeline }: CardProps) {
    const badge = asset.type === 'video' ? 'badge-video' :
        asset.type === 'audio' ? 'badge-audio' : 'badge-image';
    const icon = asset.type === 'video' ? '🎬' :
        asset.type === 'audio' ? '🎵' : '🖼️';

    return (
        <div
            className={`asset-card${selected ? ' selected' : ''}`}
            onClick={onSelect}
            onDoubleClick={onAddToTimeline}
        >
            <div className="asset-thumb">
                {asset.thumbnailUrl
                    ? <img src={asset.thumbnailUrl} alt={asset.filename} draggable={false} />
                    : <span className="no-thumb">{icon}</span>
                }
            </div>
            <div className="asset-info">
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                    <span className={`badge ${badge}`}>{asset.type}</span>
                    <button
                        style={{ background: 'transparent', color: 'var(--text-dim)', fontSize: 12, padding: '0 2px' }}
                        onClick={e => { e.stopPropagation(); onDelete(); }}
                        title="Remove asset"
                    >✕</button>
                </div>
                <div className="asset-name" title={asset.filename}>{asset.filename}</div>
                <div className="asset-dur">{formatDur(asset.duration)}</div>
            </div>
        </div>
    );
}
