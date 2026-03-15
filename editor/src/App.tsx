import { Editor } from './pages/Editor';

export default function App() {
  return (
    <div className="relative w-full h-full overflow-hidden">
      <div className="crt-scanline"></div>
      <Editor />
    </div>
  );
}

