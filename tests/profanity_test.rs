use synoid_core::agent::specialized::smart_editor::filter_ops::*;
use synoid_core::agent::transcription::TranscriptSegment;

#[test]
fn test_get_profanity_word_list_new_keywords() {
    let list = get_profanity_word_list();
    assert!(list.contains(&"george floyd"));
    assert!(list.contains(&"georgefloyd"));
    assert!(list.contains(&"floyd"));
    assert!(list.contains(&"nigger"));
    assert!(list.contains(&"nigga"));
    assert!(list.contains(&"fag"));
    assert!(list.contains(&"faggy"));
}

#[test]
fn test_word_boundary_match_regex() {
    // Single word prefix matching
    assert!(word_boundary_match("this is fucking crazy", "fuck"));
    assert!(word_boundary_match("don't be a dickhead", "dick"));
    
    // Exact word boundary matching for phrases
    assert!(word_boundary_match("Remember George Floyd", "george floyd"));
    assert!(word_boundary_match("georgefloyd was a name", "georgefloyd"));
    
    // Case insensitivity
    assert!(word_boundary_match("GEORGE FLOYD", "george floyd"));
    assert!(word_boundary_match("FUCK", "fuck"));
    
    // Negative cases
    assert!(!word_boundary_match("classy", "ass"));
    assert!(!word_boundary_match("The grass is green", "ass"));
}

#[test]
fn test_estimate_word_timestamps_multiple_occurrences() {
    let seg = TranscriptSegment {
        start: 0.0,
        end: 10.0,
        text: "fuck this shit and fuck that too".to_string(),
    };

    // Test multiple occurrences of the same bad word
    let fuck_timestamps = estimate_word_timestamps(&seg, "fuck");
    assert_eq!(fuck_timestamps.len(), 2, "Should find two 'fuck's");

    // "fuck" is word 0 and word 4 of 7 words total.
    // Index 0: estimated at 0.0 + (0/7)*10 = 0.0, with 150ms pre-pad → starts at 0.0 (clamped)
    // Index 4: estimated at 0.0 + (4/7)*10 = 5.71, with 150ms pre-pad → starts around 5.56

    // Beep should start AT or BEFORE the word (with pre-padding)
    assert!(fuck_timestamps[0].0 <= 0.15, "First beep should start at or near 0.0 (segment start)");
    assert!(fuck_timestamps[1].0 < 5.71 && fuck_timestamps[1].0 > 5.4, "Second beep should start before word at ~5.56");

    // Beep should extend AFTER the word (with post-padding)
    assert!(fuck_timestamps[0].1 > 1.4, "First beep should cover the word");
    assert!(fuck_timestamps[1].1 > 8.1, "Second beep should cover the word");

    // Test single occurrence of another bad word
    let shit_timestamps = estimate_word_timestamps(&seg, "shit");
    assert_eq!(shit_timestamps.len(), 1, "Should find one 'shit'");
    // "shit" is word 2: estimated at 0.0 + (2/7)*10 = 2.857, with 150ms pre-pad → starts around 2.707
    assert!(shit_timestamps[0].0 < 2.857, "Beep should start BEFORE the estimated word position");
    assert!(shit_timestamps[0].0 > 2.5, "Beep should be close to word position");
}

#[test]
fn test_estimate_word_timestamps_phrase() {
    let seg = TranscriptSegment {
        start: 0.0,
        end: 10.0,
        text: "Justice for George Floyd now".to_string(),
    };
    
    let timestamps = estimate_word_timestamps(&seg, "george floyd");
    // Since it's a phrase, it falls back to the entire segment because "george floyd" 
    // doesn't match single tokens "george" or "floyd" in the word-by-word loop.
    assert_eq!(timestamps.len(), 1);
    assert_eq!(timestamps[0], (0.0, 10.0));
}
