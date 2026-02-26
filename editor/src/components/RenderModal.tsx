import React, { useEffect, useRef } from 'react';
import type { RenderStatus } from '../types';

interface Props {
    sessionId: string | null;
    onClose: () => void;
    onDownload?: (path: string) => void;
}

export function RenderModal({ sessionId, onClose, onDownload }: Props) {
    const [status, setStatus] = React.useState<RenderStatus>({ progress: 0, status: 'idle' });
    const intervalRef = useRef<ReturnType<typeof setInterval>>();

    useEffect(() => {
        if (!sessionId) return;

        const poll = async () => {
            try {
                const res = await fetch(`/api/editor/sessions/${sessionId}/render/status`);
                const data: RenderStatus = await res.json();
                setStatus(data);
                if (data.status === 'done' || data.status === 'error') {
                    clearInterval(intervalRef.current);
                }
            } catch { }
        };

        poll();
        intervalRef.current = setInterval(poll, 1000);
        return () => clearInterval(intervalRef.current);
    }, [sessionId]);

    const pct = Math.round(status.progress * 100);

    return (
        <div className="render-overlay">
            <div className="render-card">
                <div className="render-title">
                    {status.status === 'done' ? '✅ Export Complete' :
                        status.status === 'error' ? '❌ Export Failed' :
                            '⏳ Exporting…'}
                </div>

                <div className="progress-bar-bg">
                    <div
                        className="progress-bar-fill"
                        style={{ width: `${status.status === 'rendering' ? Math.max(5, pct) : pct}%` }}
                    />
                </div>

                <div className="render-status-text">
                    {status.status === 'rendering' && `Processing… ${pct}%`}
                    {status.status === 'idle' && 'Starting render…'}
                    {status.status === 'done' && `Output: ${status.outputPath ?? 'Ready'}`}
                    {status.status === 'error' && (status.error || 'An error occurred')}
                </div>

                {status.status === 'done' && status.outputPath && (
                    <a
                        href={`/api/editor/sessions/${sessionId}/render/output`}
                        download
                        style={{
                            display: 'block', padding: '8px 16px', textAlign: 'center',
                            background: 'var(--accent-green)', color: '#fff', borderRadius: 6,
                            fontWeight: 600, fontSize: 13, textDecoration: 'none',
                        }}
                        onClick={() => onDownload?.(status.outputPath!)}
                    >
                        ⬇ Download Video
                    </a>
                )}

                {(status.status === 'done' || status.status === 'error') && (
                    <button onClick={onClose} className="render-done-btn" style={{ alignSelf: 'flex-end' }}>
                        Close
                    </button>
                )}
            </div>
        </div>
    );
}
