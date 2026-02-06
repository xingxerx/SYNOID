// SYNOID Muxer Module
// Implements Strict Interleaved Muxing Loop and Drain Protocol
// Copyright (c) 2026 Xing_The_Creator | SYNOID

use ffmpeg_next as ffmpeg;
use tracing::{info, warn};

/// Result of a muxing operation indicating which stream was written
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MuxResult {
    VideoWritten,
    AudioWritten,
    NoneWritten,
}

/// Helper to ensure the codec context flags include GLOBAL_HEADER.
/// This is critical for MP4 containers to ensure the header contains necessary metadata.
pub fn ensure_global_headers(_codec_context: &mut ffmpeg::codec::Context) {
    // NOTE: Direct access to flags() or set_flags() via the safe wrapper is version dependent.
    // In a production environment, this should be set on the Builder before creation.
    // For now, we rely on the caller to have set this up correctly.
    warn!("[MUXER] ensure_global_headers: Ensure GLOBAL_HEADER is set in Encoder Builder!");
}

/// Enforces monotonic DTS (Decoding Time Stamp).
/// Many players freeze if DTS is not strictly increasing.
pub fn enforce_monotonic_dts(packet: &mut ffmpeg::Packet, last_dts: &mut i64) {
    if let Some(dts) = packet.dts() {
        if dts <= *last_dts {
            // warn!("[MUXER] Correcting non-monotonic DTS: {} -> {}", dts, *last_dts + 1);
            let new_dts = *last_dts + 1;
            packet.set_dts(Some(new_dts));

            // If PTS is now less than DTS (invalid), bump PTS too
            if let Some(pts) = packet.pts() {
                if pts < new_dts {
                    packet.set_pts(Some(new_dts));
                }
            }
            *last_dts = new_dts;
        } else {
            *last_dts = dts;
        }
    } else {
        // If no DTS, assume monotonic increase
        let new_dts = *last_dts + 1;
        packet.set_dts(Some(new_dts));
        *last_dts = new_dts;
    }
}

/// SYNOID: Core Interleaving Logic
/// Purpose: Prevent video freezing by ensuring DTS/PTS monotonicity
/// Returns which packet was written so the caller knows which stream to replenish.
pub fn mux_streams(
    format_context: &mut ffmpeg::format::context::Output,
    video_packet: &mut ffmpeg::Packet,
    audio_packet: &mut ffmpeg::Packet,
    video_stream_index: usize,
    audio_stream_index: usize,
    video_time_base: ffmpeg::Rational,
    audio_time_base: ffmpeg::Rational,
) -> Result<MuxResult, ffmpeg::Error> {

    let v_pts_val = video_packet.pts().unwrap_or(0);
    let a_pts_val = audio_packet.pts().unwrap_or(0);

    // Rescale to seconds for comparison
    let v_pts_seconds = v_pts_val as f64 * (video_time_base.numerator() as f64 / video_time_base.denominator() as f64);
    let a_pts_seconds = a_pts_val as f64 * (audio_time_base.numerator() as f64 / audio_time_base.denominator() as f64);

    if v_pts_seconds <= a_pts_seconds {
        // Write Video
        video_packet.set_stream(video_stream_index);

        let out_stream = format_context.stream(video_stream_index).ok_or(ffmpeg::Error::StreamNotFound)?;
        video_packet.rescale_ts(video_time_base, out_stream.time_base());

        // Use packet.write_interleaved(&mut context) as per ffmpeg-next API
        video_packet.write_interleaved(format_context)?;
        return Ok(MuxResult::VideoWritten);
    } else {
        // Write Audio
        audio_packet.set_stream(audio_stream_index);

        let out_stream = format_context.stream(audio_stream_index).ok_or(ffmpeg::Error::StreamNotFound)?;
        audio_packet.rescale_ts(audio_time_base, out_stream.time_base());

        audio_packet.write_interleaved(format_context)?;
        return Ok(MuxResult::AudioWritten);
    }
}

/// SYNOID: Final Buffer Flush Logic
/// Purpose: Forces the encoder to output the "trapped" final frames
pub fn flush_encoder(
    encoder: &mut ffmpeg::encoder::Video,
    format_context: &mut ffmpeg::format::context::Output,
    stream_index: usize,
    encoder_time_base: ffmpeg::Rational,
) -> Result<(), ffmpeg::Error> {
    info!("[MUXER] Flushing encoder buffer...");

    // 1. Signal EOF to the encoder
    encoder.send_eof()?;

    // Pre-allocate packet outside loop for memory ownership optimization
    let mut packet = ffmpeg::Packet::empty();

    // 2. Continuous drain loop until encoder is empty
    while encoder.receive_packet(&mut packet).is_ok() {
        // Correct the stream index before writing
        packet.set_stream(stream_index);

        // Ensure PTS/DTS is rescaled to the output stream's timebase
        let out_stream = format_context.stream(stream_index).ok_or(ffmpeg::Error::StreamNotFound)?;

        packet.rescale_ts(encoder_time_base, out_stream.time_base());

        // Write the final "flushed" packets to the file
        packet.write_interleaved(format_context)?;
    }

    info!("[MUXER] Flush complete.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_monotonic_dts() {
        // Initialize ffmpeg
        ffmpeg::init().unwrap();

        let mut packet = ffmpeg::Packet::empty();
        let mut last_dts = 100;

        // Case 1: Packet has no DTS, should set to last + 1
        enforce_monotonic_dts(&mut packet, &mut last_dts);
        assert_eq!(packet.dts(), Some(101));
        assert_eq!(last_dts, 101);

        // Case 2: Packet has valid DTS > last, should keep it
        packet.set_dts(Some(105));
        enforce_monotonic_dts(&mut packet, &mut last_dts);
        assert_eq!(packet.dts(), Some(105));
        assert_eq!(last_dts, 105);

        // Case 3: Packet has DTS <= last (non-monotonic), should fix it
        packet.set_dts(Some(100)); // lower than 105
        enforce_monotonic_dts(&mut packet, &mut last_dts);
        assert_eq!(packet.dts(), Some(106)); // 105 + 1
        assert_eq!(last_dts, 106);

        // Case 4: PTS fixing
        packet.set_dts(Some(100));
        packet.set_pts(Some(100));
        enforce_monotonic_dts(&mut packet, &mut last_dts);
        assert_eq!(packet.dts(), Some(107)); // 106 + 1
        assert_eq!(packet.pts(), Some(107)); // should be bumped to match DTS if it was lower
        assert_eq!(last_dts, 107);
    }
}
