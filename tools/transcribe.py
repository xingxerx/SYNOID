import whisper
import argparse
import json
import os
import sys
import torch

def get_safe_device():
    """Detect compatible GPU. RTX 50 series (Blackwell sm_120) not yet supported by PyTorch."""
    if not torch.cuda.is_available():
        return "cpu"
    
    try:
        # Check CUDA compute capability
        device_props = torch.cuda.get_device_properties(0)
        major, minor = device_props.major, device_props.minor
        compute_cap = major * 10 + minor
        
        # PyTorch currently supports up to sm_90 (RTX 40 series)
        # RTX 50 series (Blackwell) is sm_120 - not yet supported
        if compute_cap >= 120:
            gpu_name = device_props.name
            print(f"[PY] WARNING: {gpu_name} (sm_{compute_cap}) not yet supported by PyTorch.", file=sys.stderr)
            print(f"[PY] Falling back to CPU mode for reliable transcription.", file=sys.stderr)
            return "cpu"
        
        return "cuda"
    except Exception as e:
        print(f"[PY] GPU detection failed: {e}, using CPU", file=sys.stderr)
        return "cpu"

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--audio", required=True)
    parser.add_argument("--model", default="tiny")
    parser.add_argument("--output", required=True)
    parser.add_argument("--force-cpu", action="store_true", help="Force CPU mode")
    args = parser.parse_args()

    device = "cpu" if args.force_cpu else get_safe_device()
    print(f"[PY] Loading Whisper model: {args.model} on {device}...", file=sys.stderr)
    
    try:
        model = whisper.load_model(args.model, device=device)
    except Exception as e:
        print(f"[PY] Failed to load model: {e}", file=sys.stderr)
        # If CUDA failed, try CPU as last resort
        if device == "cuda":
            print(f"[PY] Retrying with CPU...", file=sys.stderr)
            try:
                model = whisper.load_model(args.model, device="cpu")
                device = "cpu"
            except Exception as e2:
                print(f"[PY] CPU fallback also failed: {e2}", file=sys.stderr)
                sys.exit(1)
        else:
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

    print(f"[PY] Transcription saved to {args.output} ({len(segments)} segments)", file=sys.stderr)

if __name__ == "__main__":
    main()

