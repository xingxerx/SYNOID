// SYNOID Reference-Guided Editor
// Copyright (c) 2026 xingxerx_The_Creator | SYNOID
//
// Supports dual-mode editing with reference image guidance
// Combines instruction-based editing with reference image guidance

use crate::agent::engines::process_utils::CommandExt;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::process::Command;
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EditMode {
    /// Instruction-only editing using natural language
    InstructionOnly { intent: String },

    /// Reference-guided editing using a reference image
    ReferenceGuided {
        intent: String,
        reference_image: PathBuf,
    },

    /// Dual-mode: Combines both instruction and reference
    Dual {
        intent: String,
        reference_image: PathBuf,
        blend_strength: f32, // 0.0 = instruction only, 1.0 = reference only
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EditingTask {
    /// Global style transfer (cartoon, sketch, watercolor, etc.)
    GlobalStyle { style: String },

    /// Background replacement
    BackgroundChange { description: String },

    /// Local object modification
    LocalChange {
        object: String,
        modification: String,
    },

    /// Object/person removal
    LocalRemove { object: String },

    /// Add new objects to scene
    LocalAdd {
        object: String,
        position: Option<String>,
    },
}

/// Configuration for reference-guided editing
#[derive(Debug, Clone)]
pub struct ReferenceEditConfig {
    pub mode: EditMode,
    pub task: EditingTask,
    pub temporal_consistency: f32, // 0.0-1.0, higher = more consistent across frames
    pub reference_strength: f32,   // How strongly to apply reference guidance
    pub preserve_motion: bool,     // Maintain original video motion
}

impl Default for ReferenceEditConfig {
    fn default() -> Self {
        Self {
            mode: EditMode::InstructionOnly {
                intent: String::new(),
            },
            task: EditingTask::GlobalStyle {
                style: "none".to_string(),
            },
            temporal_consistency: 0.85,
            reference_strength: 0.7,
            preserve_motion: true,
        }
    }
}

pub struct ReferenceEditor {
    _api_url: String,
    _model: String,
}

impl ReferenceEditor {
    pub fn new(api_url: &str, model: &str) -> Self {
        Self {
            _api_url: api_url.to_string(),
            _model: model.to_string(),
        }
    }

    /// Main entry point for reference-guided editing
    pub async fn apply_reference_edit(
        &self,
        input_video: &Path,
        output_video: &Path,
        config: ReferenceEditConfig,
    ) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
        info!("[REF-EDIT] Starting reference-guided editing");
        info!("[REF-EDIT] Mode: {:?}", config.mode);
        info!("[REF-EDIT] Task: {:?}", config.task);

        match &config.mode {
            EditMode::InstructionOnly { intent } => {
                self.instruction_only_edit(input_video, output_video, intent, &config)
                    .await
            }
            EditMode::ReferenceGuided {
                intent,
                reference_image,
            } => {
                self.reference_guided_edit(
                    input_video,
                    output_video,
                    intent,
                    reference_image,
                    &config,
                )
                .await
            }
            EditMode::Dual {
                intent,
                reference_image,
                blend_strength,
            } => {
                self.dual_mode_edit(
                    input_video,
                    output_video,
                    intent,
                    reference_image,
                    *blend_strength,
                    &config,
                )
                .await
            }
        }
    }

    /// Instruction-only editing (existing SYNOID smart editor approach)
    async fn instruction_only_edit(
        &self,
        input: &Path,
        output: &Path,
        intent: &str,
        _config: &ReferenceEditConfig,
    ) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
        info!("[REF-EDIT] Instruction-only mode: {}", intent);

        // Delegate to existing smart editor
        // This maintains backward compatibility
        let _result = crate::agent::smart_editor::smart_edit(
            input, intent, output, false,
            None, None, None, None, None,
            true, // enable_subtitles
            true, // enable_censoring
        )
        .await?;

        Ok(output.to_path_buf())
    }

    /// Reference-guided editing using reference image for visual control
    async fn reference_guided_edit(
        &self,
        input: &Path,
        output: &Path,
        _intent: &str,
        reference_image: &Path,
        config: &ReferenceEditConfig,
    ) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
        info!("[REF-EDIT] Reference-guided mode");
        info!("[REF-EDIT] Reference image: {:?}", reference_image);

        if !reference_image.exists() {
            return Err(format!("Reference image not found: {:?}", reference_image).into());
        }

        // Extract visual features from reference image using vision API
        let reference_features = self.extract_reference_features(reference_image).await?;

        // Apply reference-guided transformation
        match &config.task {
            EditingTask::GlobalStyle { style: _ } => {
                self.apply_style_transfer(input, output, &reference_features, config)
                    .await
            }
            EditingTask::BackgroundChange { description } => {
                self.apply_background_change(
                    input,
                    output,
                    &reference_features,
                    description,
                    config,
                )
                .await
            }
            EditingTask::LocalChange {
                object,
                modification,
            } => {
                self.apply_local_change(
                    input,
                    output,
                    &reference_features,
                    object,
                    modification,
                    config,
                )
                .await
            }
            EditingTask::LocalRemove { object } => {
                self.apply_local_remove(input, output, object, config).await
            }
            EditingTask::LocalAdd { object, position } => {
                self.apply_local_add(
                    input,
                    output,
                    &reference_features,
                    object,
                    position.as_deref(),
                    config,
                )
                .await
            }
        }
    }

    /// Dual-mode: Combines instruction and reference guidance
    async fn dual_mode_edit(
        &self,
        input: &Path,
        output: &Path,
        intent: &str,
        reference_image: &Path,
        blend_strength: f32,
        config: &ReferenceEditConfig,
    ) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
        info!(
            "[REF-EDIT] Dual-mode editing (blend: {:.2})",
            blend_strength
        );

        let work_dir = input
            .parent()
            .unwrap_or(Path::new("."))
            .join(".synoid_work");
        std::fs::create_dir_all(&work_dir)?;

        // Step 1: Apply instruction-based edit
        let instruction_output = work_dir.join("dual_instruction.mp4");
        self.instruction_only_edit(input, &instruction_output, intent, config)
            .await?;

        // Step 2: Apply reference-guided edit
        let reference_output = work_dir.join("dual_reference.mp4");
        self.reference_guided_edit(input, &reference_output, intent, reference_image, config)
            .await?;

        // Step 3: Blend the two results based on blend_strength
        self.blend_videos(
            &instruction_output,
            &reference_output,
            output,
            blend_strength,
        )
        .await?;

        // Cleanup intermediate files
        let _ = std::fs::remove_file(instruction_output);
        let _ = std::fs::remove_file(reference_output);

        Ok(output.to_path_buf())
    }

    /// Extract visual features from reference image
    async fn extract_reference_features(
        &self,
        reference_image: &Path,
    ) -> Result<ReferenceFeatures, Box<dyn std::error::Error + Send + Sync>> {
        info!(
            "[REF-EDIT] Extracting reference features from {:?}",
            reference_image
        );

        // Use vision API to analyze reference image
        let vision_result = crate::agent::vision_tools::analyze_image_gemini(
            reference_image,
            "Describe the visual style, colors, composition, lighting, and artistic elements of this image in detail."
        ).await?;

        Ok(ReferenceFeatures {
            description: vision_result,
            image_path: reference_image.to_path_buf(),
        })
    }

    /// Apply style transfer based on reference
    async fn apply_style_transfer(
        &self,
        input: &Path,
        output: &Path,
        reference: &ReferenceFeatures,
        config: &ReferenceEditConfig,
    ) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
        info!("[REF-EDIT] Applying style transfer with reference");

        // Use FFmpeg's advanced filters for style-like transformations
        // In production, this would call a neural style transfer model
        let style_filter =
            self.build_style_filter(&reference.description, config.temporal_consistency);

        let status = Command::new("ffmpeg")
            .stealth()
            .args(["-i"])
            .arg(input)
            .args(["-vf", &style_filter])
            .args(["-c:v", "libx264", "-preset", "medium", "-crf", "18"])
            .args(["-c:a", "copy"])
            .arg(output)
            .args(["-y"])
            .status()
            .await?;

        if !status.success() {
            return Err("Style transfer failed".into());
        }

        Ok(output.to_path_buf())
    }

    /// Build FFmpeg filter string for style transfer
    fn build_style_filter(&self, style_description: &str, temporal_consistency: f32) -> String {
        // Parse style description to extract visual properties
        let lower = style_description.to_lowercase();

        let mut filters = Vec::new();

        // Temporal consistency via minterpolate
        if temporal_consistency > 0.5 {
            filters.push(format!("minterpolate=fps=30:mi_mode=blend"));
        }

        // Style-based filters
        if lower.contains("cartoon") || lower.contains("anime") {
            filters.push("edgedetect=mode=colormix:high=0.3".to_string());
            filters.push("hue=s=1.3".to_string());
        } else if lower.contains("sketch") || lower.contains("drawing") {
            filters.push("edgedetect=mode=canny:low=0.1:high=0.4".to_string());
            filters.push("colorchannelmixer=.3:.4:.3:0:.3:.4:.3:0:.3:.4:.3".to_string());
        } else if lower.contains("watercolor") {
            filters.push("smartblur=lr=1:ls=-0.5".to_string());
            filters.push("eq=saturation=1.2:brightness=0.05".to_string());
        } else if lower.contains("vibrant") || lower.contains("saturated") {
            filters.push("eq=saturation=1.5:contrast=1.1".to_string());
        } else if lower.contains("muted") || lower.contains("desaturated") {
            filters.push("eq=saturation=0.7:contrast=0.9".to_string());
        }

        if filters.is_empty() {
            "null".to_string()
        } else {
            filters.join(",")
        }
    }

    /// Apply background change
    async fn apply_background_change(
        &self,
        input: &Path,
        output: &Path,
        reference: &ReferenceFeatures,
        description: &str,
        config: &ReferenceEditConfig,
    ) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
        info!("[REF-EDIT] Applying background change: {}", description);

        // Use chromakey or advanced segmentation
        // For now, use a simple approach with FFmpeg
        let filter = format!(
            "chromakey=color=green:similarity=0.3:blend=0.1,{}",
            if config.preserve_motion {
                "null"
            } else {
                "scale=1920:1080"
            }
        );

        let status = Command::new("ffmpeg")
            .stealth()
            .args(["-i"])
            .arg(input)
            .args(["-i"])
            .arg(&reference.image_path)
            .args(["-filter_complex", &filter])
            .args(["-c:v", "libx264", "-preset", "medium", "-crf", "18"])
            .arg(output)
            .args(["-y"])
            .status()
            .await?;

        if !status.success() {
            return Err("Background change failed".into());
        }

        Ok(output.to_path_buf())
    }

    /// Apply local change to specific object
    async fn apply_local_change(
        &self,
        input: &Path,
        output: &Path,
        _reference: &ReferenceFeatures,
        object: &str,
        modification: &str,
        config: &ReferenceEditConfig,
    ) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
        info!("[REF-EDIT] Local change: {} -> {}", object, modification);

        // This requires object detection and segmentation
        // For MVP, apply global filter
        warn!("[REF-EDIT] Local change requires ML model - falling back to instruction edit");

        let intent = format!("Change {} to {}", object, modification);
        self.instruction_only_edit(input, output, &intent, config)
            .await
    }

    /// Remove object from video
    async fn apply_local_remove(
        &self,
        input: &Path,
        output: &Path,
        object: &str,
        config: &ReferenceEditConfig,
    ) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
        info!("[REF-EDIT] Removing object: {}", object);

        // Object removal requires inpainting
        warn!("[REF-EDIT] Object removal requires ML model - falling back to instruction edit");

        let intent = format!("Remove {} from the video", object);
        self.instruction_only_edit(input, output, &intent, config)
            .await
    }

    /// Add object to video
    async fn apply_local_add(
        &self,
        input: &Path,
        output: &Path,
        _reference: &ReferenceFeatures,
        object: &str,
        position: Option<&str>,
        config: &ReferenceEditConfig,
    ) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
        info!("[REF-EDIT] Adding object: {} at {:?}", object, position);

        // Object addition requires compositing
        warn!("[REF-EDIT] Object addition requires ML model - falling back to instruction edit");

        let intent = if let Some(pos) = position {
            format!("Add {} at {}", object, pos)
        } else {
            format!("Add {}", object)
        };

        self.instruction_only_edit(input, output, &intent, config)
            .await
    }

    /// Blend two videos based on strength parameter
    async fn blend_videos(
        &self,
        video1: &Path,
        video2: &Path,
        output: &Path,
        blend_strength: f32,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!(
            "[REF-EDIT] Blending videos with strength {:.2}",
            blend_strength
        );

        let blend_filter = format!(
            "[0:v][1:v]blend=all_mode=normal:all_opacity={}",
            blend_strength
        );

        let status = Command::new("ffmpeg")
            .stealth()
            .args(["-i"])
            .arg(video1)
            .args(["-i"])
            .arg(video2)
            .args(["-filter_complex", &blend_filter])
            .args(["-c:v", "libx264", "-preset", "medium", "-crf", "18"])
            .args(["-c:a", "copy"])
            .arg(output)
            .args(["-y"])
            .status()
            .await?;

        if !status.success() {
            return Err("Video blending failed".into());
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
struct ReferenceFeatures {
    description: String,
    image_path: PathBuf,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_edit_mode_creation() {
        let mode = EditMode::InstructionOnly {
            intent: "Make it look cinematic".to_string(),
        };
        assert!(matches!(mode, EditMode::InstructionOnly { .. }));
    }

    #[test]
    fn test_style_filter_building() {
        let editor = ReferenceEditor::new("http://localhost:11434", "llama3");
        let filter = editor.build_style_filter("cartoon anime vibrant colors", 0.8);
        assert!(filter.contains("edgedetect") || filter.contains("hue"));
    }
}
