import React, { useCallback, useRef, useState } from 'react';
import type { AIChatMessage, AIAction } from '../types';
import * as api from '../api';

interface Props {
    sessionId: string | null;
    assetId?: string | null;
    onAction?: (action: AIAction) => void;
}

export function DirectorPanel({ sessionId, assetId, onAction }: Props) {
    const [messages, setMessages] = useState<AIChatMessage[]>([
        {
            role: 'assistant',
            content: ":: INITIALIZING_DIRECTOR_KERNEL...\n:: AWAITING_CMD: [ SILENCE_REMOVAL | CAPTION_GEN | ACTION_HIGHLIGHT ]\n:: SYSTEM_READY.",
            timestamp: Date.now(),
        }
    ]);
    const [input, setInput] = useState('');
    const [loading, setLoading] = useState(false);
    const bottomRef = useRef<HTMLDivElement>(null);

    const scrollToBottom = () => {
        setTimeout(() => bottomRef.current?.scrollIntoView({ behavior: 'smooth' }), 50);
    };

    const send = useCallback(async () => {
        if (!input.trim() || !sessionId || loading) return;

        const userMsg: AIChatMessage = { role: 'user', content: input, timestamp: Date.now() };
        setMessages(prev => [...prev, userMsg]);
        setInput('');
        setLoading(true);
        scrollToBottom();

        try {
            const res = await api.aiChat(sessionId, input);
            const assistantMsg: AIChatMessage = {
                role: 'assistant',
                content: res.response,
                timestamp: Date.now(),
                actions: res.actions as AIAction[],
            };
            setMessages(prev => [...prev, assistantMsg]);
        } catch (e: any) {
            setMessages(prev => [...prev, {
                role: 'assistant',
                content: `!! ERROR: ${e.message}\n!! CHECK_DEPENDENCY: [ OLLAMA ]\n!! CMD: [ ollama serve ]`,
                timestamp: Date.now(),
            }]);
        } finally {
            setLoading(false);
            scrollToBottom();
        }
    }, [input, sessionId, loading]);

    const handleKey = (e: React.KeyboardEvent) => {
        if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); send(); }
    };

    const executeAction = useCallback(async (action: AIAction) => {
        if (!sessionId) return;
        if (action.type === 'auto-edit' || action.type === 'transcribe') {
            onAction?.(action);
        }
    }, [sessionId, onAction]);

    return (
        <div style={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
            {/* Messages */}
            <div style={{ flex: 1, overflowY: 'auto', paddingBottom: 8 }}>
                <div className="chat-messages">
                    {messages.map((msg, i) => (
                        <div key={i} className={`chat-msg ${msg.role}`}>
                            {msg.role === 'assistant' && <div className="msg-label">:: SYNOID::DIRECTOR_PROMPT</div>}
                            <div style={{ whiteSpace: 'pre-wrap', wordBreak: 'break-word' }}>{msg.content}</div>
                            {msg.actions && msg.actions.length > 0 && (
                                <div className="chat-actions">
                                    {msg.actions.map((action, j) => (
                                        <button
                                            key={j}
                                            className="chat-action-btn"
                                            onClick={() => executeAction(action)}
                                        >
                                            [ ▷ ] {action.label.toUpperCase()}
                                        </button>
                                    ))}
                                </div>
                            )}
                        </div>
                    ))}
                    {loading && (
                        <div className="chat-msg assistant">
                            <div className="msg-label">:: SYNOID::DIRECTOR_PROMPT</div>
                            <span className="spinner" />
                        </div>
                    )}
                    <div ref={bottomRef} />
                </div>
            </div>

            {/* Input */}
            <div className="chat-input-area" style={{ padding: '8px 0 0', borderTop: '1px solid var(--border-dim)' }}>
                <textarea
                    className="chat-textarea"
                    placeholder="INPUT_COMMANDS_HERE... e.g. 'EXTRACT_HIGHLIGHTS'"
                    value={input}
                    onChange={e => setInput(e.target.value)}
                    onKeyDown={handleKey}
                    rows={3}
                    disabled={loading || !sessionId}
                />
                <button
                    className="send-btn"
                    onClick={send}
                    disabled={loading || !input.trim() || !sessionId}
                >
                    {loading ? '...' : '[ ⚡ ] RUN_COMMIT'}
                </button>
            </div>
        </div>
    );
}
