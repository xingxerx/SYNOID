import sys
import json
import os
import warnings

# Suppress warnings
warnings.filterwarnings("ignore")

def install_whisper():
    import subprocess
    print("Installing openai-whisper...", file=sys.stderr)
    subprocess.check_call([sys.executable, "-m", "pip", "install", "openai-whisper"])

try:
    import whisper
except ImportError:
    install_whisper()
    import whisper

def transcribe(video_path):
    if not os.path.exists(video_path):
        print(json.dumps({"error": "File not found"}))
        return

    # Load model (use 'base' for speed)
    print(f"Loading Whisper model...", file=sys.stderr)
    model = whisper.load_model("base")

    print(f"Transcribing {video_path}...", file=sys.stderr)
    result = model.transcribe(video_path)

    # Format output as compact JSON
    output = {
        "text": result["text"],
        "segments": [
            {
                "start": seg["start"],
                "end": seg["end"],
                "text": seg["text"].strip()
            }
            for seg in result["segments"]
        ],
        "language": result["language"]
    }

    print(json.dumps(output))

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print(json.dumps({"error": "Usage: python transcribe_bridge.py <video_path>"}))
        sys.exit(1)
    
    video_path = sys.argv[1]
    transcribe(video_path)
