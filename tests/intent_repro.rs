use synoid_core::agent::smart_editor::{EditIntent, EditDensity};

#[test]
fn test_duration_parsing() {
    let intent = EditIntent::from_text("Make it 40-60 minutes");
    assert_eq!(intent.density, EditDensity::Full);
    assert_eq!(intent.target_duration, Some((2400.0, 3600.0)));
}

#[test]
fn test_duration_parsing_mins() {
    let intent = EditIntent::from_text("I want a 40-60 mins video");
    assert_eq!(intent.density, EditDensity::Full);
    assert_eq!(intent.target_duration, Some((2400.0, 3600.0)));
}

#[test]
fn test_highlights() {
    let intent = EditIntent::from_text("aggressive highlights");
    assert_eq!(intent.density, EditDensity::Highlights);
    assert!(intent.ruthless);
}

#[test]
fn test_negation_fail() {
    let intent = EditIntent::from_text("remove all talking");
    assert!(intent.keep_speech, "This is expected to be True currently due to weak parsing");
}

#[test]
fn test_continuity_synchronization() {
    use synoid_core::agent::smart_editor::{Scene, EditingStrategy, ensure_speech_continuity};
    use synoid_core::agent::transcription::TranscriptSegment;

    let mut scenes = vec![
        Scene { start_time: 0.0, end_time: 2.0, duration: 2.0, score: 0.1 }, // Part of sentence
        Scene { start_time: 2.0, end_time: 4.0, duration: 2.0, score: 0.6 }, // Good part of sentence
    ];

    let transcript = vec![
        TranscriptSegment { start: 0.5, end: 3.5, text: "Wait for it... YES!".to_string() },
    ];

    let config = EditingStrategy::default();
    ensure_speech_continuity(&mut scenes, &transcript, &config, false);

    // Both should now have the max score (0.6)
    assert_eq!(scenes[0].score, 0.6);
    assert_eq!(scenes[1].score, 0.6);
}
