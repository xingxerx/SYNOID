import { useCallback, useRef, useState, type ReactNode } from 'react';

interface Props {
  left: ReactNode;
  center: ReactNode;
  right: ReactNode;
  defaultLeftWidth?: number;
  defaultRightWidth?: number;
  minWidth?: number;
}

export function ResizablePanel({
  left, center, right,
  defaultLeftWidth = 240,
  defaultRightWidth = 300,
  minWidth = 180,
}: Props) {
  const [leftW, setLeftW] = useState(defaultLeftWidth);
  const [rightW, setRightW] = useState(defaultRightWidth);
  const dragging = useRef<'left' | 'right' | null>(null);

  const onMouseDown = useCallback((side: 'left' | 'right') => (e: React.MouseEvent) => {
    e.preventDefault();
    dragging.current = side;
    const startX = e.clientX;
    const startLeft = leftW;
    const startRight = rightW;

    const onMove = (ev: MouseEvent) => {
      const dx = ev.clientX - startX;
      if (dragging.current === 'left') {
        setLeftW(Math.max(minWidth, startLeft + dx));
      } else {
        setRightW(Math.max(minWidth, startRight - dx));
      }
    };
    const onUp = () => {
      dragging.current = null;
      document.removeEventListener('mousemove', onMove);
      document.removeEventListener('mouseup', onUp);
      document.body.style.cursor = '';
      document.body.classList.remove('no-select');
    };

    document.body.style.cursor = 'col-resize';
    document.body.classList.add('no-select');
    document.addEventListener('mousemove', onMove);
    document.addEventListener('mouseup', onUp);
  }, [leftW, rightW, minWidth]);

  return (
    <div className="flex h-full w-full overflow-hidden">
      {/* Left Panel */}
      <div className="flex-shrink-0 overflow-y-auto bg-synoid-sidebar border-r border-synoid-border"
           style={{ width: leftW }}>
        {left}
      </div>

      {/* Left Resize Handle */}
      <div className="w-1 flex-shrink-0 cursor-col-resize bg-synoid-border hover:bg-synoid-orange transition-colors"
           onMouseDown={onMouseDown('left')} />

      {/* Center Panel */}
      <div className="flex-1 min-w-0 overflow-hidden flex flex-col bg-synoid-bg">
        {center}
      </div>

      {/* Right Resize Handle */}
      <div className="w-1 flex-shrink-0 cursor-col-resize bg-synoid-border hover:bg-synoid-orange transition-colors"
           onMouseDown={onMouseDown('right')} />

      {/* Right Panel */}
      <div className="flex-shrink-0 overflow-y-auto bg-synoid-sidebar border-l border-synoid-border"
           style={{ width: rightW }}>
        {right}
      </div>
    </div>
  );
}
