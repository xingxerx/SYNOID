use super::types::Scene;
use std::path::{Path, PathBuf};
// SYNOID Smart Editor Refactoring

fn build_smooth_xfade_filter(
    scenes: &[Scene],
    audio_input_idx: usize,
    transition_duration: f64,
) -> String {
    let n = scenes.len();
    if n < 2 {
        return String::new();
    }

    let effects = ["fade", "wipeleft", "wiperight", "slideleft", "slideright"];
    let mut filter = String::new();

    // Step 1: Trim all segments
    for (i, scene) in scenes.iter().enumerate() {
        filter.push_str(&format!(
            "[0:v]trim=start={:.6}:end={:.6},setpts=PTS-STARTPTS[vraw{i}]; ",
            scene.start_time, scene.end_time
        ));
        filter.push_str(&format!(
            "[{audio_input_idx}:a]atrim=start={:.6}:end={:.6},asetpts=PTS-STARTPTS[araw{i}]; ",
            scene.start_time, scene.end_time
        ));
    }

    // Step 2: Chain xfade transitions for video
    let mut prev_v = "vraw0".to_string();
    let mut offset = scenes[0].duration - transition_duration;

    for i in 1..n {
        let effect = effects[i % effects.len()];
        let out_label = if i == n - 1 {
            "outv".to_string()
        } else {
            format!("vx{i}")
        };
        filter.push_str(&format!(
            "[{prev_v}][vraw{i}]xfade=transition={effect}:duration={:.3}:offset={:.6}[{out_label}]; ",
            transition_duration, offset.max(0.0)
        ));
        prev_v = out_label;
        // Next offset accounts for the current segment minus the overlap
        offset += scenes[i].duration - transition_duration;
    }

    // Step 3: Chain acrossfade for audio
    let mut prev_a = "araw0".to_string();
    for i in 1..n {
        let out_label = if i == n - 1 {
            "outa".to_string()
        } else {
            format!("ax{i}")
        };
        let dur = transition_duration
            .min(scenes[i].duration * 0.5)
            .min(scenes[i - 1].duration * 0.5);
        filter.push_str(&format!(
            "[{prev_a}][araw{i}]acrossfade=d={:.3}:c1=tri:c2=tri[{out_label}]; ",
            dur
        ));
        prev_a = out_label;
    }

    // Remove trailing "; "
    if filter.ends_with("; ") {
        filter.truncate(filter.len() - 2);
    }

    filter
}

