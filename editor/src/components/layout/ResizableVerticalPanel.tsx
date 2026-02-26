import { useCallback, useRef, useState, type ReactNode } from 'react';

interface Props {
  top: ReactNode;
  bottom: ReactNode;
  defaultTopRatio?: number;
  minHeight?: number;
}

export function ResizableVerticalPanel({
  top, bottom,
  defaultTopRatio = 0.55,
  minHeight = 120,
}: Props) {
  const containerRef = useRef<HTMLDivElement>(null);
  const [ratio, setRatio] = useState(defaultTopRatio);

  const onMouseDown = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    const startY = e.clientY;
    const startRatio = ratio;
    const container = containerRef.current;
    if (!container) return;
    const h = container.getBoundingClientRect().height;

    const onMove = (ev: MouseEvent) => {
      const dy = ev.clientY - startY;
      const newRatio = startRatio + dy / h;
      const minR = minHeight / h;
      const maxR = 1 - minR;
      setRatio(Math.max(minR, Math.min(maxR, newRatio)));
    };
    const onUp = () => {
      document.removeEventListener('mousemove', onMove);
      document.removeEventListener('mouseup', onUp);
      document.body.style.cursor = '';
      document.body.classList.remove('no-select');
    };

    document.body.style.cursor = 'row-resize';
    document.body.classList.add('no-select');
    document.addEventListener('mousemove', onMove);
    document.addEventListener('mouseup', onUp);
  }, [ratio, minHeight]);

  return (
    <div ref={containerRef} className="flex flex-col h-full w-full overflow-hidden">
      <div className="overflow-hidden" style={{ height: `${ratio * 100}%` }}>
        {top}
      </div>
      <div className="h-1 flex-shrink-0 cursor-row-resize bg-synoid-border hover:bg-synoid-orange transition-colors"
           onMouseDown={onMouseDown} />
      <div className="flex-1 min-h-0 overflow-hidden">
        {bottom}
      </div>
    </div>
  );
}
