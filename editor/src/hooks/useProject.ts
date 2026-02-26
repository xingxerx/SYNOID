import { useCallback, useEffect, useRef, useState } from 'react';
import type {
  Asset, TimelineClip, Track, CaptionData, ProjectSettings, RightPanelTab,
  CaptionWord, CaptionStyle, ClipTransform,
} from '../types';
import { DEFAULT_TRACKS, DEFAULT_PROJECT_SETTINGS, DEFAULT_CAPTION_STYLE, DEFAULT_TRANSFORM } from '../types';
import * as api from '../api';

function uid(): string {
  return crypto.randomUUID?.() ?? `${Date.now()}-${Math.random().toString(36).slice(2)}`;
}

function formatTime(s: number): string {
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const sec = Math.floor(s % 60);
  const fr = Math.floor((s % 1) * 30);
  return `${String(h).padStart(2, '0')}:${String(m).padStart(2, '0')}:${String(sec).padStart(2, '0')}:${String(fr).padStart(2, '0')}`;
}

export function useProject() {
  // ─── Core State ───────────────────────────────────────────────────────
  const [sessionId, setSessionId] = useState<string | null>(null);
  const [assets, setAssets] = useState<Asset[]>([]);
  const [tracks] = useState<Track[]>(DEFAULT_TRACKS);
  const [clips, setClips] = useState<TimelineClip[]>([]);
  const [captionData, setCaptionData] = useState<Record<string, CaptionData>>({});
  const [settings] = useState<ProjectSettings>(DEFAULT_PROJECT_SETTINGS);

  // ─── UI State ─────────────────────────────────────────────────────────
  const [selectedClipId, setSelectedClipId] = useState<string | null>(null);
  const [playheadPosition, setPlayheadPosition] = useState(0);
  const [isPlaying, setIsPlaying] = useState(false);
  const [timelineZoom, setTimelineZoom] = useState(40); // pixels per second
  const [rightPanelTab, setRightPanelTab] = useState<RightPanelTab>('director');
  const [isTranscribing, setIsTranscribing] = useState(false);

  // ─── Refs for async operations ────────────────────────────────────────
  const clipsRef = useRef(clips);
  const captionDataRef = useRef(captionData);
  const assetsRef = useRef(assets);
  useEffect(() => { clipsRef.current = clips; }, [clips]);
  useEffect(() => { captionDataRef.current = captionData; }, [captionData]);
  useEffect(() => { assetsRef.current = assets; }, [assets]);

  // ─── Computed ─────────────────────────────────────────────────────────
  const duration = clips.reduce((max, c) => Math.max(max, c.start + c.duration), 0);
  const selectedClip = clips.find(c => c.id === selectedClipId) ?? null;
  const playheadTime = formatTime(playheadPosition);
  const durationTime = formatTime(duration);

  // ─── Session ──────────────────────────────────────────────────────────
  useEffect(() => {
    const stored = localStorage.getItem('synoid-session');
    if (stored) {
      api.getSession(stored)
        .then(() => setSessionId(stored))
        .catch(() => {
          localStorage.removeItem('synoid-session');
          createSession();
        });
    } else {
      createSession();
    }
  }, []);

  async function createSession() {
    try {
      const s = await api.createSession();
      setSessionId(s.id);
      localStorage.setItem('synoid-session', s.id);
    } catch {
      // Backend not running - work in offline mode
      setSessionId('offline-' + uid());
    }
  }

  // ─── Asset Operations ─────────────────────────────────────────────────
  const uploadAsset = useCallback(async (file: File) => {
    if (!sessionId) return;
    try {
      const asset = await api.uploadAsset(sessionId, file);
      setAssets(prev => [...prev, asset]);
      return asset;
    } catch (e) {
      console.error('Upload failed:', e);
      // Offline fallback: create local asset
      const localAsset: Asset = {
        id: uid(),
        type: file.type.startsWith('video/') ? 'video' : file.type.startsWith('audio/') ? 'audio' : 'image',
        filename: file.name,
        duration: 0,
        width: 1920,
        height: 1080,
        size: file.size,
        thumbnailUrl: null,
        streamUrl: URL.createObjectURL(file),
      };
      setAssets(prev => [...prev, localAsset]);
      return localAsset;
    }
  }, [sessionId]);

  const deleteAsset = useCallback(async (assetId: string) => {
    if (sessionId) api.deleteAsset(sessionId, assetId).catch(() => {});
    setAssets(prev => prev.filter(a => a.id !== assetId));
    setClips(prev => prev.filter(c => c.assetId !== assetId));
  }, [sessionId]);

  // ─── Clip Operations ──────────────────────────────────────────────────
  const addClip = useCallback((assetId: string, trackId: string, start: number, duration?: number) => {
    const asset = assetsRef.current.find(a => a.id === assetId);
    const dur = duration ?? asset?.duration ?? 5;
    const clip: TimelineClip = {
      id: uid(),
      assetId,
      trackId,
      start,
      duration: dur,
      inPoint: 0,
      outPoint: dur,
      transform: { ...DEFAULT_TRANSFORM },
      speed: 1,
      volume: 1,
    };
    setClips(prev => [...prev, clip]);
    setSelectedClipId(clip.id);
    return clip;
  }, []);

  const updateClip = useCallback((clipId: string, updates: Partial<TimelineClip>) => {
    setClips(prev => prev.map(c => c.id === clipId ? { ...c, ...updates } : c));
  }, []);

  const deleteClip = useCallback((clipId: string) => {
    setClips(prev => prev.filter(c => c.id !== clipId));
    setCaptionData(prev => {
      const next = { ...prev };
      delete next[clipId];
      return next;
    });
    if (selectedClipId === clipId) setSelectedClipId(null);
  }, [selectedClipId]);

  const splitClip = useCallback((clipId: string, time: number) => {
    setClips(prev => {
      const idx = prev.findIndex(c => c.id === clipId);
      if (idx === -1) return prev;
      const clip = prev[idx];
      const relativeTime = time - clip.start;
      if (relativeTime <= 0.05 || relativeTime >= clip.duration - 0.05) return prev;

      const left: TimelineClip = {
        ...clip,
        duration: relativeTime,
        outPoint: clip.inPoint + relativeTime / clip.speed,
      };
      const right: TimelineClip = {
        ...clip,
        id: uid(),
        start: time,
        duration: clip.duration - relativeTime,
        inPoint: clip.inPoint + relativeTime / clip.speed,
      };
      const next = [...prev];
      next[idx] = left;
      next.push(right);
      return next;
    });
  }, []);

  const moveClip = useCallback((clipId: string, newStart: number, newTrackId?: string) => {
    setClips(prev => prev.map(c => {
      if (c.id !== clipId) return c;
      return { ...c, start: Math.max(0, newStart), ...(newTrackId ? { trackId: newTrackId } : {}) };
    }));
  }, []);

  // ─── Caption Operations ───────────────────────────────────────────────
  const transcribeAsset = useCallback(async (assetId: string) => {
    if (!sessionId) return;
    setIsTranscribing(true);
    try {
      const result = await api.transcribeAsset(sessionId, assetId);
      const asset = assetsRef.current.find(a => a.id === assetId);
      if (!asset) return;

      // Create caption clip on T1
      const clipId = uid();
      const captionClip: TimelineClip = {
        id: clipId,
        assetId: '',
        trackId: 'T1',
        start: 0,
        duration: asset.duration || result.words[result.words.length - 1]?.end || 10,
        inPoint: 0,
        outPoint: asset.duration || 10,
        transform: { ...DEFAULT_TRANSFORM },
        speed: 1,
        volume: 1,
      };
      setClips(prev => [...prev, captionClip]);
      setCaptionData(prev => ({
        ...prev,
        [clipId]: {
          words: result.words,
          style: { ...DEFAULT_CAPTION_STYLE },
        },
      }));
    } catch (e) {
      console.error('Transcription failed:', e);
    } finally {
      setIsTranscribing(false);
    }
  }, [sessionId]);

  const updateCaptionStyle = useCallback((clipId: string, style: Partial<CaptionStyle>) => {
    setCaptionData(prev => {
      const existing = prev[clipId];
      if (!existing) return prev;
      return { ...prev, [clipId]: { ...existing, style: { ...existing.style, ...style } } };
    });
  }, []);

  const updateCaptionWords = useCallback((clipId: string, words: CaptionWord[]) => {
    setCaptionData(prev => {
      const existing = prev[clipId];
      if (!existing) return prev;
      return { ...prev, [clipId]: { ...existing, words } };
    });
  }, []);

  // ─── Playback ─────────────────────────────────────────────────────────
  const playPause = useCallback(() => setIsPlaying(p => !p), []);

  const seek = useCallback((time: number) => {
    setPlayheadPosition(Math.max(0, Math.min(time, duration || Infinity)));
  }, [duration]);

  const skipBack = useCallback(() => seek(Math.max(0, playheadPosition - 5)), [seek, playheadPosition]);
  const skipForward = useCallback(() => seek(playheadPosition + 5), [seek, playheadPosition]);

  // ─── Timeline Zoom ────────────────────────────────────────────────────
  const zoomIn = useCallback(() => setTimelineZoom(z => Math.min(200, z * 1.3)), []);
  const zoomOut = useCallback(() => setTimelineZoom(z => Math.max(5, z / 1.3)), []);

  // ─── Keyboard Shortcuts ───────────────────────────────────────────────
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) return;
      switch (e.key) {
        case ' ':
          e.preventDefault();
          playPause();
          break;
        case 'Delete':
        case 'Backspace':
          if (selectedClipId) deleteClip(selectedClipId);
          break;
        case 's':
          if (!e.ctrlKey && selectedClipId) splitClip(selectedClipId, playheadPosition);
          break;
        case '=':
        case '+':
          zoomIn();
          break;
        case '-':
          zoomOut();
          break;
      }
    };
    window.addEventListener('keydown', handler);
    return () => window.removeEventListener('keydown', handler);
  }, [playPause, deleteClip, splitClip, selectedClipId, playheadPosition, zoomIn, zoomOut]);

  return {
    // State
    sessionId, assets, tracks, clips, captionData, settings,
    selectedClipId, selectedClip, playheadPosition, isPlaying,
    timelineZoom, rightPanelTab, isTranscribing,
    duration, playheadTime, durationTime,
    // Actions
    uploadAsset, deleteAsset,
    addClip, updateClip, deleteClip, splitClip, moveClip,
    transcribeAsset, updateCaptionStyle, updateCaptionWords,
    setSelectedClipId, setPlayheadPosition: seek, playPause,
    skipBack, skipForward, zoomIn, zoomOut,
    setRightPanelTab, setIsPlaying,
  };
}
