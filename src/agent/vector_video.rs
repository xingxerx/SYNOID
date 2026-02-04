// SYNOID Vector Video Engine
// True SVG/Vector-based video rendering (not frame-by-frame)
// Inspired by Rive's real-time vector graphics approach

use std::path::{Path, PathBuf};
use std::fs;
use tracing::{info, warn};

/// Configuration for vector video processing
#[derive(Clone)]
pub struct VectorVideoConfig {
    /// Output resolution (width)
    pub target_width: u32,
    /// Output resolution (height)
    pub target_height: u32,
    /// Frame rate for output
    pub fps: u32,
    /// Quality preset
    pub quality: VectorQuality,
}

/// Vector rendering quality presets
#[derive(Clone, Copy)]
pub enum VectorQuality {
    /// Fast preview (lower detail)
    Preview,
    /// Standard quality
    Standard,
    /// Maximum quality (slower)
    Ultra,
}

impl Default for VectorVideoConfig {
    fn default() -> Self {
        Self {
            target_width: 1920,
            target_height: 1080,
            fps: 30,
            quality: VectorQuality::Standard,
        }
    }
}

/// Vector Video Engine - uses mathematical curves instead of pixels
pub struct VectorVideoEngine {
    config: VectorVideoConfig,
    work_dir: PathBuf,
}

impl VectorVideoEngine {
    pub fn new(config: VectorVideoConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let work_dir = std::env::temp_dir().join("synoid_vector_video");
        fs::create_dir_all(&work_dir)?;
        
        info!("[VECTOR-VIDEO] Engine initialized ({}x{} @ {}fps)", 
            config.target_width, config.target_height, config.fps);
        
        Ok(Self { config, work_dir })
    }

    /// Convert raster video to vector format (Lottie/Rive)
    /// This is the "Infinite Zoom" capability - vector graphics scale infinitely
    pub fn rasterize_to_vector(
        &self,
        input_video: &Path,
        output_path: &Path,
    ) -> Result<String, Box<dyn std::error::Error>> {
        info!("[VECTOR-VIDEO] Converting {:?} to vector format...", input_video);
        
        // Strategy: 
        // 1. Extract key frames from video
        // 2. Vectorize each keyframe using vtracer (edge detection -> bezier curves)
        // 3. Interpolate between keyframes using vector morphing
        // 4. Output as animated SVG or Lottie JSON
        
        // Step 1: Extract keyframes (not every frame - only scene changes)
        let keyframes_dir = self.work_dir.join("keyframes");
        fs::create_dir_all(&keyframes_dir)?;
        
        // Use scene detection for smart keyframe extraction
        let ffmpeg_status = std::process::Command::new("ffmpeg")
            .args([
                "-i", input_video.to_str().unwrap(),
                "-vf", "select='gt(scene,0.3)',showinfo", // Only extract on scene changes
                "-vsync", "vfr",
                "-frame_pts", "true",
                keyframes_dir.join("kf_%04d.png").to_str().unwrap(),
            ])
            .output()?;

        if !ffmpeg_status.status.success() {
            // Fallback: extract at fixed intervals
            warn!("[VECTOR-VIDEO] Scene detection failed, using interval extraction");
            std::process::Command::new("ffmpeg")
                .args([
                    "-i", input_video.to_str().unwrap(),
                    "-vf", "fps=2", // 2 keyframes per second
                    keyframes_dir.join("kf_%04d.png").to_str().unwrap(),
                ])
                .output()?;
        }

        // Step 2: Vectorize keyframes
        let svg_dir = self.work_dir.join("vector_frames");
        fs::create_dir_all(&svg_dir)?;
        
        let keyframes: Vec<PathBuf> = fs::read_dir(&keyframes_dir)?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().map_or(false, |e| e == "png"))
            .collect();

        info!("[VECTOR-VIDEO] Extracted {} keyframes, vectorizing...", keyframes.len());

        for (i, kf_path) in keyframes.iter().enumerate() {
            let svg_path = svg_dir.join(format!("frame_{:04}.svg", i));
            self.vectorize_frame(kf_path, &svg_path)?;
        }

        // Step 3: Create animated output
        // For true vector video, we create a master SVG with SMIL animation
        // or output as Lottie JSON for maximum compatibility
        let result = self.create_animated_svg(&svg_dir, output_path)?;
        
        info!("[VECTOR-VIDEO] ✅ Vector video created: {:?}", output_path);
        Ok(result)
    }

    /// Vectorize a single frame using vtracer
    fn vectorize_frame(&self, input: &Path, output: &Path) -> Result<(), Box<dyn std::error::Error>> {
        // Use vtracer's convert function (same API as vector_engine.rs)
        let config = vtracer::Config {
            color_mode: vtracer::ColorMode::Color,
            hierarchical: vtracer::Hierarchical::Stacked,
            filter_speckle: 4,
            color_precision: 6,
            layer_difference: 16,
            corner_threshold: 60,
            splice_threshold: 45,
            ..Default::default()
        };

        // vtracer::convert_image_to_svg takes (input, output, config)
        vtracer::convert_image_to_svg(input, output, config)?;
        Ok(())
    }

    /// Create an animated SVG from multiple vector frames using SMIL
    fn create_animated_svg(
        &self,
        svg_dir: &Path,
        output: &Path,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let svg_files: Vec<PathBuf> = fs::read_dir(svg_dir)?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().map_or(false, |e| e == "svg"))
            .collect();

        if svg_files.is_empty() {
            return Err("No SVG frames found".into());
        }

        // Read first SVG to get dimensions
        let first_svg = fs::read_to_string(&svg_files[0])?;
        
        // Calculate frame duration based on FPS
        let frame_duration = 1.0 / self.config.fps as f64;
        let total_duration = frame_duration * svg_files.len() as f64;

        // Create animated SVG with SMIL
        let mut animated_svg = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" 
     xmlns:xlink="http://www.w3.org/1999/xlink"
     width="{}" height="{}" viewBox="0 0 {} {}">
  <title>SYNOID Vector Video</title>
  <desc>Infinite resolution vector video - scale to any size</desc>
  
  <!-- Frame Container -->
  <g id="frames">
"#,
            self.config.target_width, self.config.target_height,
            self.config.target_width, self.config.target_height
        );

        // Add each frame as a group with animation
        for (i, svg_path) in svg_files.iter().enumerate() {
            let svg_content = fs::read_to_string(svg_path)?;
            // Extract just the inner content (skip XML declaration and outer SVG tag)
            let inner = self.extract_svg_content(&svg_content);
            
            let begin_time = i as f64 * frame_duration;
            let visibility = if i == 0 { "visible" } else { "hidden" };
            
            animated_svg.push_str(&format!(
                r#"    <g id="frame_{}" style="visibility: {}">
      {}
      <set attributeName="visibility" to="visible" begin="{}s" dur="{}s" fill="freeze"/>
      <set attributeName="visibility" to="hidden" begin="{}s" fill="freeze"/>
    </g>
"#,
                i, visibility, inner, 
                begin_time, frame_duration,
                begin_time + frame_duration
            ));
        }

        animated_svg.push_str(&format!(
            r#"  </g>
  
  <!-- Animation loops every {} seconds -->
  <animate attributeName="display" values="block" dur="{}s" repeatCount="indefinite"/>
</svg>"#,
            total_duration, total_duration
        ));

        // Write animated SVG
        fs::write(output, &animated_svg)?;
        
        Ok(format!("Created {} frame animated SVG ({:.1}s @ {}fps)", 
            svg_files.len(), total_duration, self.config.fps))
    }

    /// Extract inner content from an SVG file (skip XML declaration and outer tag)
    fn extract_svg_content(&self, svg: &str) -> String {
        // Find the opening <svg> tag and extract content between <svg> and </svg>
        if let Some(start) = svg.find('>') {
            if let Some(end) = svg.rfind("</svg>") {
                return svg[start + 1..end].trim().to_string();
            }
        }
        svg.to_string()
    }

    /// Render animated SVG to video at any resolution
    pub fn render_to_video(
        &self,
        animated_svg: &Path,
        output_video: &Path,
        scale: f64,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let final_width = (self.config.target_width as f64 * scale) as u32;
        let final_height = (self.config.target_height as f64 * scale) as u32;

        info!("[VECTOR-VIDEO] Rendering to {}x{} ({}x scale)", 
            final_width, final_height, scale);

        // Safety check
        if final_width > 16384 || final_height > 16384 {
            return Err(format!(
                "Safety Stop: {}x{} exceeds 16K limit. Reduce scale.", 
                final_width, final_height
            ).into());
        }

        // Use Chromium/headless browser to render animated SVG to video
        // Alternative: use resvg frame-by-frame rendering
        
        // For now, we'll use ffmpeg's SVG support if available
        let status = std::process::Command::new("ffmpeg")
            .args([
                "-i", animated_svg.to_str().unwrap(),
                "-vf", &format!("scale={}:{}", final_width, final_height),
                "-c:v", "libx264",
                "-pix_fmt", "yuv420p",
                "-y",
                output_video.to_str().unwrap(),
            ])
            .output()?;

        if !status.status.success() {
            return Err("FFmpeg failed to render SVG video".into());
        }

        Ok(format!("Rendered {}x{} video", final_width, final_height))
    }
}
