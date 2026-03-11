import os

lines = open("d:/SYNOID/src/agent/smart_editor.rs", "r", encoding="utf-8").readlines()

def write_out(filename, ranges):
    os.makedirs(os.path.dirname(filename), exist_ok=True)
    with open(filename, "w", encoding="utf-8") as f:
        f.write("// SYNOID Smart Editor Refactoring\n\n")
        for start, end in ranges:
            f.writelines(lines[start-1:end])

# types.rs
write_out("d:/SYNOID/src/agent/smart_editor/types.rs", [
    (34, 359)
])

# scene_ops.rs
write_out("d:/SYNOID/src/agent/smart_editor/scene_ops.rs", [
    (16, 16),      # const SILENCE_REFINEMENT_THRESHOLD
    (17, 17),      # use regex::Captures;
    (360, 471),    # merge_neighboring_scenes, bridge_narrative_gaps
    (653, 1123)    # detect_scenes, ensure_speech_continuity, refine_scenes_with_transcript, score_scenes, scene_has_speech
])

# filter_ops.rs
write_out("d:/SYNOID/src/agent/smart_editor/filter_ops.rs", [
    (472, 652),    # insert_cut_markers
    (2346, 2525)   # generate_srt, get_profanity, word_match, estimate
])

# transition_ops.rs
write_out("d:/SYNOID/src/agent/smart_editor/transition_ops.rs", [
    (2104, 2177)   # build_smooth_xfade_filter
])

# mod.rs
write_out("d:/SYNOID/src/agent/smart_editor/mod.rs", [
    (1, 15),       # Header + use statements
    (18, 33),      # strip_unc_prefix
    (1124, 2103),  # smart_edit
    (2178, 2345),  # fallback_extract_and_concat
    (2526, len(lines)) # tests
])

print("Files generated!")
