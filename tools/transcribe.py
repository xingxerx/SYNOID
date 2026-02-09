import argparse
import json
import os
import sys
import torch
import warnings

# Suppress warnings
warnings.filterwarnings("ignore")

def get_safe_device(force_cpu=False):
    """Detect compatible GPU. RTX 50 series (Blackwell sm_120) not yet supported by PyTorch."""
    if force_cpu or not torch.cuda.is_available():
        return "cpu"
    
    try:
        # Check CUDA compute capability
        device_props = torch.cuda.get_device_properties(0)
        major, minor = device_props.major, device_props.minor
        compute_cap = major * 10 + minor
        
        # PyTorch currently supports up to sm_90 (RTX 40 series)
        # RTX 50 series (Blackwell) is sm_120 - not yet supported
        # RTX 50 series (Blackwell) is sm_120 - Supported by newer PyTorch builds
        if compute_cap >= 120:
            gpu_name = device_props.name
            print(f"[PY] INFO: {gpu_name} (sm_{compute_cap}) detected. Attempting to use CUDA...", file=sys.stderr)
            return "cuda"
        
        return "cuda"
    except Exception as e:
        print(f"[PY] GPU detection failed: {e}, using CPU", file=sys.stderr)
        return "cpu"

def main():
    parser = argparse.ArgumentParser(description="SYNOID Whisper Transcription Wrapper")
    parser.add_argument("--input", required=True, help="Input audio/video file")
    parser.add_argument("--output", required=True, help="Output JSON file")
    parser.add_argument("--model", default="medium", help="Whisper model size")
    parser.add_argument("--force-cpu", action="store_true", help="Force CPU usage")
    parser.add_argument("--save-txt", help="Optional path to save plain text transcript")
    args = parser.parse_args()

    input_path = args.input
    output_path = args.output
    
    if not os.path.exists(input_path):
        print(f"Error: Input file not found: {input_path}", file=sys.stderr)
        sys.exit(1)

    device = get_safe_device(args.force_cpu)
    print(f"[PY] Loading Whisper model '{args.model}' on {device}...", file=sys.stderr)

    try:
        import whisper
        model = whisper.load_model(args.model, device=device)
        
        print(f"[PY] Transcribing...", file=sys.stderr)
        result = model.transcribe(input_path)
        
        segments = []
        for seg in result["segments"]:
            segments.append({
                "start": seg["start"],
                "end": seg["end"],
                "text": seg["text"].strip()
            })
        
        # Save JSON
        with open(output_path, "w", encoding="utf-8") as f:
            json.dump(segments, f, indent=2)
            
        # Save Text if requested
        if args.save_txt:
            with open(args.save_txt, "w", encoding="utf-8") as f:
                f.write(result["text"].strip())
                
        print(f"[PY] Transcription complete. {len(segments)} segments.", file=sys.stderr)
        
    except ImportError:
        print("Error: 'openai-whisper' package not installed. Run: pip install openai-whisper", file=sys.stderr)
        sys.exit(1)
    except Exception as e:
        print(f"Error during transcription: {e}", file=sys.stderr)
        sys.exit(1)

if __name__ == "__main__":
    main()
