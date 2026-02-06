use std::path::PathBuf;
use std::process::Command;
use synoid_core::agent::production_tools;

#[tokio::test]
async fn test_trim_video_integration() {
    // 1. Setup: Create a dummy video file using ffmpeg
    let input_path = PathBuf::from("test_input.mp4");
    let output_path = PathBuf::from("test_output.mp4");

    // Cleanup previous run
    if input_path.exists() {
        std::fs::remove_file(&input_path).unwrap();
    }
    if output_path.exists() {
        std::fs::remove_file(&output_path).unwrap();
    }

    // Generate 5 second video
    // testsrc generates a test pattern. lavfi is the libavfilter input virtual device.
    let status = Command::new("ffmpeg")
        .args([
            "-y",
            "-f",
            "lavfi",
            "-i",
            "testsrc=duration=5:size=1280x720:rate=30",
            "-c:v",
            "libx264",
            "-g",
            "1",
            "test_input.mp4",
        ])
        .output()
        .expect("Failed to execute ffmpeg");

    if !status.status.success() {
        eprintln!("FFmpeg stderr: {}", String::from_utf8_lossy(&status.stderr));
        panic!("Failed to create dummy video");
    }

    // 2. Execute: Trim the video (1s start, 2s duration)
    // This uses the current (blocking) implementation initially, then will verify async
    let result = production_tools::trim_video(&input_path, 1.0, 2.0, &output_path).await;

    // 3. Verify
    assert!(result.is_ok(), "trim_video failed: {:?}", result.err());

    let prod_result = result.unwrap();
    assert!(prod_result.output_path.exists());
    assert!(prod_result.size_mb > 0.0);

    // Check duration of output
    let duration = synoid_core::agent::source_tools::get_video_duration(&output_path)
        .await
        .expect("Failed to get duration");
    assert!(
        (duration - 2.0).abs() < 0.5,
        "Duration should be approx 2.0s, got {}",
        duration
    );

    // Cleanup
    let _ = std::fs::remove_file(input_path);
    let _ = std::fs::remove_file(output_path);
}

#[tokio::test]
async fn test_compress_video_integration() {
    let input_path = PathBuf::from("test_compress_in.mp4");
    let output_path = PathBuf::from("test_compress_out.mp4");

    // Cleanup
    if input_path.exists() {
        std::fs::remove_file(&input_path).unwrap();
    }
    if output_path.exists() {
        std::fs::remove_file(&output_path).unwrap();
    }

    // Generate 5 second video
    let status = Command::new("ffmpeg")
        .args([
            "-y",
            "-f",
            "lavfi",
            "-i",
            "testsrc=duration=5:size=1280x720:rate=30",
            "-c:v",
            "libx264",
            "test_compress_in.mp4",
        ])
        .output()
        .expect("Failed to execute ffmpeg");

    if !status.status.success() {
        panic!("Failed to create dummy video");
    }

    // Compress to very small size (e.g., 0.5 MB)
    let result = production_tools::compress_video(&input_path, 0.5, &output_path).await;

    assert!(result.is_ok(), "compress_video failed: {:?}", result.err());

    let prod_result = result.unwrap();
    assert!(prod_result.output_path.exists());

    // Cleanup
    let _ = std::fs::remove_file(input_path);
    let _ = std::fs::remove_file(output_path);
}
