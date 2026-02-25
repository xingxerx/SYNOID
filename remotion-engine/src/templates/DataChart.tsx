import { AbsoluteFill, useCurrentFrame, useVideoConfig, interpolate, spring } from 'remotion';
export interface DataChartProps { type?: 'bar' | 'pie' | 'progress' | 'line'; data: Array<{ label: string; value: number; color?: string }>; maxValue?: number; title?: string; }
export const DataChart: React.FC<DataChartProps> = ({ type = 'bar', data, maxValue, title }) => {
    const frame = useCurrentFrame(); const { fps } = useVideoConfig();
    const colors = ['#f97316', '#3b82f6', '#22c55e', '#8b5cf6', '#ec4899', '#eab308'];
    const actualMax = maxValue || Math.max(...data.map(d => d.value));
    return (<AbsoluteFill style={{ justifyContent: 'center', alignItems: 'center', display: 'flex', flexDirection: 'column', gap: 40, padding: 60 }}>
        {title && <div style={{ fontSize: 48, fontWeight: 'bold', color: '#fff', fontFamily: 'Inter, system-ui, sans-serif' }}>{title}</div>}
        {type === 'bar' && <div style={{ display: 'flex', alignItems: 'flex-end', gap: 24, height: 300 }}>
            {data.map((item, i) => {
                const p = spring({ frame: frame - i * 8, fps, config: { damping: 12, stiffness: 100 } }); const h = interpolate(p, [0, 1], [0, (item.value / actualMax) * 250]); const c = item.color || colors[i % colors.length];
                return (<div key={i} style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', flex: 1 }}>
                    <div style={{ fontSize: 24, fontWeight: 'bold', color: c, marginBottom: 8, opacity: interpolate(p, [0, 0.5, 1], [0, 1, 1]) }}>{item.value}</div>
                    <div style={{ width: 60, height: h, backgroundColor: c, borderRadius: '8px 8px 0 0', boxShadow: `0 0 30px ${c}66` }} />
                    <div style={{ marginTop: 12, fontSize: 16, color: '#a1a1aa', textAlign: 'center', fontFamily: 'Inter, system-ui, sans-serif' }}>{item.label}</div>
                </div>);
            })}
        </div>}
        {type === 'pie' && <div style={{ width: 300, height: 300, borderRadius: '50%', background: `conic-gradient(from -90deg, ${data.map((item, i) => { const c = item.color || colors[i % colors.length]; const start = data.slice(0, i).reduce((s, d) => s + d.value, 0); const total = data.reduce((s, d) => s + d.value, 0); return `${c} ${(start / total) * 100}% ${((start + item.value) / total) * 100}%`; }).join(', ')})`, boxShadow: '0 0 60px rgba(0,0,0,0.5)' }} />}
    </AbsoluteFill>);
};
