use synoid_core::agent::smart_editor::{score_scenes, EditIntent, EditingStrategy, Scene};
use synoid_core::agent::voice::transcription::TranscriptSegment;

#[tokio::test]
async fn test_smart_edit_fallback() {
    // 1. Setup scenes that would normally be filtered out in ruthless mode
    let mut scenes = vec![
        Scene {
            start_time: 0.0,
            end_time: 10.0,
            duration: 10.0,
            score: 0.5,
        }, // Boring/Silence
    ];

    let intent = EditIntent {
        remove_boring: true,
        keep_action: false,
        remove_silence: true,
        keep_speech: false,
        ruthless: true,
        custom_keywords: vec![],
    };

    let config = EditingStrategy::default();

    // 2. Score them
    score_scenes(&mut scenes, &intent, None, &config);

    // We expect the score to be low due to ruthless + boring
    // In ruthless mode, base 0.3 - 0.1 (ruthless) - 0.3 (boring penalty > 30s? No, config.boring_penalty_threshold is 30.0)
    // Wait, let's check config.boring_penalty_threshold = 30.0.
    // If duration < 15.0, no boring penalty currently?
    // Ah, lines 356-364:
    // if duration > 30.0 -> -0.3
    // if duration > 15.0 -> -0.15
    // if duration < 3.0 -> +0.2

    // Let's make a truly "boring" long scene
    let mut scenes = vec![Scene {
        start_time: 0.0,
        end_time: 40.0,
        duration: 40.0,
        score: 0.5,
    }];
    score_scenes(&mut scenes, &intent, None, &config);

    // Score: 0.3 (base) - 0.1 (ruthless) - 0.3 (boring) = -0.1 -> clamped to 0.0
    assert!(
        scenes[0].score < config.min_scene_score,
        "Scene should be filtered out by score. Score: {}",
        scenes[0].score
    );
}

#[tokio::test]
async fn test_speech_protection_ruthless() {
    let mut scenes = vec![Scene {
        start_time: 0.0,
        end_time: 5.0,
        duration: 5.0,
        score: 0.5,
    }];

    let transcript = vec![TranscriptSegment {
        start: 1.0,
        end: 4.0,
        text: "Hello world".to_string(),
    }];

    let intent = EditIntent {
        remove_boring: true,
        keep_action: false,
        remove_silence: true,
        keep_speech: true,
        ruthless: true,
        custom_keywords: vec![],
    };

    let config = EditingStrategy::default();

    // 2. Score them
    score_scenes(&mut scenes, &intent, Some(&transcript), &config);

    // Speech ratio = 3/5 = 0.6.
    // Score: 0.3 (base) - 0.1 (ruthless) + 0.4 (speech boost) = 0.6
    assert!(
        scenes[0].score > config.min_scene_score,
        "Speech scene should be kept even in ruthless mode. Score: {}",
        scenes[0].score
    );
}
