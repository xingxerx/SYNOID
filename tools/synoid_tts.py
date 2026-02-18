import asyncio
import argparse
import sys
import edge_tts

async def main():
    parser = argparse.ArgumentParser(description="SYNOID TTS Wrapper (edge-tts)")
    parser.add_argument("--text", required=True, help="Text to speak")
    parser.add_argument("--output", required=True, help="Output audio file (mp3/wav)")
    parser.add_argument("--voice", default="en-US-ChristopherNeural", help="Voice ID")
    parser.add_argument("--rate", default="+0%", help="Speaking rate")
    args = parser.parse_args()

    voice = args.voice
    text = args.text
    output_file = args.output
    rate = args.rate

    print(f"[TTS] Synthesizing: '{text}' using {voice}...", file=sys.stderr)

    try:
        communicate = edge_tts.Communicate(text, voice, rate=rate)
        await communicate.save(output_file)
        print(f"[TTS] Saved to {output_file}", file=sys.stderr)
    except Exception as e:
        print(f"[TTS] Error: {e}", file=sys.stderr)
        sys.exit(1)

if __name__ == "__main__":
    asyncio.run(main())
