import {
  Play, Pause, SkipBack, SkipForward, Download, Upload, Scissors,
  ZoomIn, ZoomOut, Undo2, Redo2, Settings,
} from 'lucide-react';

interface Props {
  isPlaying: boolean;
  onPlayPause: () => void;
  onSkipBack: () => void;
  onSkipForward: () => void;
  onZoomIn: () => void;
  onZoomOut: () => void;
  onSplit: () => void;
  onExport: () => void;
  onImport: () => void;
  playheadTime: string;
  duration: string;
}

function ToolBtn({ children, onClick, title, active }: {
  children: React.ReactNode; onClick?: () => void; title: string; active?: boolean;
}) {
  return (
    <button
      onClick={onClick}
      title={title}
      className={`p-1.5 rounded hover:bg-synoid-surface transition-colors ${active ? 'text-synoid-orange' : 'text-synoid-text-secondary hover:text-synoid-text-primary'}`}
    >
      {children}
    </button>
  );
}

export function Toolbar({
  isPlaying, onPlayPause, onSkipBack, onSkipForward,
  onZoomIn, onZoomOut, onSplit, onExport, onImport,
  playheadTime, duration,
}: Props) {
  return (
    <div className="h-10 flex items-center px-3 gap-1 bg-synoid-panel border-b border-synoid-border select-none">
      {/* Left: Logo + File ops */}
      <div className="flex items-center gap-2 mr-4">
        <span className="text-synoid-orange font-bold text-sm tracking-wider">SYNOID</span>
        <div className="w-px h-5 bg-synoid-border" />
        <ToolBtn onClick={onImport} title="Import Media"><Upload size={16} /></ToolBtn>
      </div>

      {/* Center: Playback controls */}
      <div className="flex-1 flex items-center justify-center gap-1">
        <ToolBtn onClick={onZoomOut} title="Zoom Out"><ZoomOut size={16} /></ToolBtn>
        <div className="w-px h-5 bg-synoid-border mx-1" />
        <ToolBtn onClick={() => {}} title="Undo"><Undo2 size={16} /></ToolBtn>
        <ToolBtn onClick={() => {}} title="Redo"><Redo2 size={16} /></ToolBtn>
        <div className="w-px h-5 bg-synoid-border mx-1" />
        <ToolBtn onClick={onSkipBack} title="Skip Back"><SkipBack size={16} /></ToolBtn>
        <button
          onClick={onPlayPause}
          title={isPlaying ? 'Pause' : 'Play'}
          className="p-2 rounded-full bg-synoid-orange text-white hover:brightness-110 transition mx-1"
        >
          {isPlaying ? <Pause size={16} /> : <Play size={16} />}
        </button>
        <ToolBtn onClick={onSkipForward} title="Skip Forward"><SkipForward size={16} /></ToolBtn>
        <div className="w-px h-5 bg-synoid-border mx-1" />
        <ToolBtn onClick={onSplit} title="Split Clip (S)"><Scissors size={16} /></ToolBtn>
        <div className="w-px h-5 bg-synoid-border mx-1" />
        <ToolBtn onClick={onZoomIn} title="Zoom In"><ZoomIn size={16} /></ToolBtn>
      </div>

      {/* Right: Time + Export */}
      <div className="flex items-center gap-3">
        <span className="text-xs font-mono text-synoid-text-secondary">
          {playheadTime} / {duration}
        </span>
        <button
          onClick={onExport}
          className="flex items-center gap-1.5 px-3 py-1 rounded bg-synoid-orange text-white text-xs font-medium hover:brightness-110 transition"
        >
          <Download size={14} />
          Export
        </button>
      </div>
    </div>
  );
}
