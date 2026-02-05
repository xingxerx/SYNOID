import whisper
import argparse
import json
import os
import sys
import torch

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--audio", required=True)
    parser.add_argument("--model", default="tiny")
    parser.add_argument("--output", required=True)
    args = parser.parse_args()

    device = "cuda" if torch.cuda.is_available() else "cpu"
    print(f"[PY] Loading Whisper model: {args.model} on {device}...", file=sys.stderr)
    
    try:
        model = whisper.load_model(args.model, device=device)
    except Exception as e:
        print(f"[PY] Failed to load model: {e}", file=sys.stderr)
        sys.exit(1)

    print(f"[PY] Transcribing {args.audio}...", file=sys.stderr)
    try:
        result = model.transcribe(args.audio)
    except Exception as e:
        print(f"[PY] Transcription failed: {e}", file=sys.stderr)
        sys.exit(1)

    segments = []
    for seg in result["segments"]:
        segments.append({
            "start": seg["start"],
            "end": seg["end"],
            "text": seg["text"].strip()
        })

    with open(args.output, "w", encoding="utf-8") as f:
        json.dump(segments, f, indent=2)

    print(f"[PY] Transcription saved to {args.output}", file=sys.stderr)

if __name__ == "__main__":
    main()
