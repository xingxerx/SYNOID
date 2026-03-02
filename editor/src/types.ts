// ─── Asset ─────────────────────────────────────────────────────────────────
export interface Asset {
  id: string;
  type: 'video' | 'image' | 'audio';
  filename: string;
  duration: number;
  width: number;
  height: number;
  size: number;
  thumbnailUrl: string | null;
  streamUrl: string;
  aiGenerated?: boolean;
}

// ─── Timeline Track ────────────────────────────────────────────────────────
export type TrackType = 'text' | 'video' | 'audio';

export interface Track {
  id: string;
  type: TrackType;
  name: string;
  order: number;
  muted: boolean;
  locked: boolean;
}

export const DEFAULT_TRACKS: Track[] = [
  { id: 'T1', type: 'text',  name: 'Captions', order: 0, muted: false, locked: false },
  { id: 'V3', type: 'video', name: 'Overlay 2', order: 1, muted: false, locked: false },
  { id: 'V2', type: 'video', name: 'Overlay 1', order: 2, muted: false, locked: false },
  { id: 'V1', type: 'video', name: 'Main Video', order: 3, muted: false, locked: false },
  { id: 'A1', type: 'audio', name: 'Audio 1', order: 4, muted: false, locked: false },
  { id: 'A2', type: 'audio', name: 'Audio 2', order: 5, muted: false, locked: false },
];

// ─── Timeline Clip ─────────────────────────────────────────────────────────
export interface ClipTransform {
  x: number;
  y: number;
  scale: number;
  rotation: number;
  opacity: number;
  cropTop: number;
  cropBottom: number;
  cropLeft: number;
  cropRight: number;
}

export const DEFAULT_TRANSFORM: ClipTransform = {
  x: 0, y: 0, scale: 1, rotation: 0, opacity: 1,
  cropTop: 0, cropBottom: 0, cropLeft: 0, cropRight: 0,
};

export interface TimelineClip {
  id: string;
  assetId: string;          // '' for caption clips
  trackId: string;
  start: number;            // position on timeline (seconds)
  duration: number;
  inPoint: number;          // source start
  outPoint: number;         // source end
  transform: ClipTransform;
  speed: number;
  volume: number;
}

// ─── Captions ──────────────────────────────────────────────────────────────
export interface CaptionWord {
  text: string;
  start: number;            // relative to clip start
  end: number;
}

export type CaptionAnimation = 'none' | 'karaoke' | 'fade' | 'pop' | 'bounce' | 'typewriter';
export type CaptionPosition = 'bottom' | 'center' | 'top';

export interface CaptionStyle {
  fontFamily: string;
  fontSize: number;
  fontWeight: 'normal' | 'bold' | 'black';
  color: string;
  backgroundColor: string;
  strokeColor: string;
  strokeWidth: number;
  position: CaptionPosition;
  animation: CaptionAnimation;
  highlightColor: string;
  timeOffset: number;
}

export const DEFAULT_CAPTION_STYLE: CaptionStyle = {
  fontFamily: 'Inter',
  fontSize: 48,
  fontWeight: 'bold',
  color: '#ffffff',
  backgroundColor: 'rgba(0,0,0,0.6)',
  strokeColor: '#000000',
  strokeWidth: 2,
  position: 'bottom',
  animation: 'karaoke',
  highlightColor: '#ff7832',
  timeOffset: 0,
};

export interface CaptionData {
  words: CaptionWord[];
  style: CaptionStyle;
}

// ─── Project ───────────────────────────────────────────────────────────────
export interface ProjectSettings {
  width: number;
  height: number;
  fps: number;
  name: string;
}

export const DEFAULT_PROJECT_SETTINGS: ProjectSettings = {
  width: 1920,
  height: 1080,
  fps: 30,
  name: 'Untitled Project',
};

export interface ProjectState {
  sessionId: string;
  assets: Asset[];
  tracks: Track[];
  clips: TimelineClip[];
  captionData: Record<string, CaptionData>;  // clipId → CaptionData
  settings: ProjectSettings;
  selectedClipId: string | null;
  playheadPosition: number;
  isPlaying: boolean;
  timelineZoom: number;
  duration: number;
}

// ─── AI ────────────────────────────────────────────────────────────────────
export interface AIChatMessage {
  role: 'user' | 'assistant';
  content: string;
  timestamp: number;
  actions?: AIAction[];
}

export interface AIAction {
  type: string;
  label: string;
  params: Record<string, unknown>;
}

// ─── API Response Types ────────────────────────────────────────────────────
export interface TranscribeResponse {
  segments: { start: number; end: number; text: string }[];
  words: CaptionWord[];
}

export interface SessionResponse {
  id: string;
  status: string;
}

export interface RenderStatus {
  progress: number;
  status: 'idle' | 'rendering' | 'done' | 'error';
  outputPath?: string;
  error?: string;
}

// ─── Right Panel Tabs ──────────────────────────────────────────────────────
export type RightPanelTab = 'director' | 'properties' | 'captions' | 'motion';
