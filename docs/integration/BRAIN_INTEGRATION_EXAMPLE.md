# Brain Integration Example - CUDA-Agent in SYNOID

## How to Use CUDA Kernels from the Brain

Here's how the SYNOID Brain can leverage CUDA kernels for high-performance video processing:

### Basic Integration

```rust
// In src/agent/brain.rs

use crate::agent::cuda_pipeline::CudaPipeline;
use std::sync::Arc;

pub struct Brain {
    // ... existing fields
    cuda_pipeline: Option<Arc<CudaPipeline>>,
}

impl Brain {
    pub fn new(/* existing params */) -> Self {
        // Initialize CUDA pipeline
        let cache_dir = std::env::current_dir()
            .unwrap_or_default()
            .join(".synoid_cuda_cache");

        let cuda_pipeline = Some(Arc::new(CudaPipeline::new(cache_dir, 0)));

        Self {
            // ... existing fields
            cuda_pipeline,
        }
    }

    pub async fn process_with_intent(&mut self, intent: &str, input: &Path, output: &Path) -> Result<()> {
        // 1. Parse intent for CUDA keywords
        let needs_color_grading = intent.contains("cinematic")
            || intent.contains("color")
            || intent.contains("grade");

        let needs_denoise = intent.contains("clean")
            || intent.contains("denoise")
            || intent.contains("noise");

        let needs_sharpen = intent.contains("sharp")
            || intent.contains("crisp")
            || intent.contains("enhance");

        // 2. Apply CUDA effects as needed
        let mut current_input = input.to_path_buf();
        let temp_dir = input.parent().unwrap().join(".synoid_temp");
        std::fs::create_dir_all(&temp_dir)?;

        if let Some(ref cuda) = self.cuda_pipeline {
            // Color grading first
            if needs_color_grading {
                let temp1 = temp_dir.join("cuda_graded.mp4");
                cuda.apply_color_grading(&current_input, &temp1, 0.9).await?;
                current_input = temp1;
            }

            // Then denoise
            if needs_denoise {
                let temp2 = temp_dir.join("cuda_denoised.mp4");
                cuda.apply_denoise(&current_input, &temp2, 0.6).await?;
                current_input = temp2;
            }

            // Finally sharpen
            if needs_sharpen {
                let temp3 = temp_dir.join("cuda_sharpened.mp4");
                cuda.apply_sharpen(&current_input, &temp3, 1.2).await?;
                current_input = temp3;
            }
        }

        // 3. Continue with smart editing
        self.motor_cortex.execute_smart_render(
            intent,
            &current_input,
            output,
            /* ... other params */
        ).await?;

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);

        Ok(())
    }
}
```

### Advanced: Intent-Based CUDA Selection

```rust
impl Brain {
    pub async fn apply_cuda_effects_from_intent(
        &self,
        intent: &str,
        input: &Path,
        output: &Path,
    ) -> Result<PathBuf> {
        let cuda = match &self.cuda_pipeline {
            Some(c) => c,
            None => return Ok(input.to_path_buf()),
        };

        // Analyze intent for multiple effects
        let effects = self.parse_cuda_intent(intent);

        let mut current = input.to_path_buf();
        let temp_dir = input.parent().unwrap().join(".synoid_cuda_temp");
        std::fs::create_dir_all(&temp_dir)?;

        for (i, effect) in effects.iter().enumerate() {
            let temp_out = temp_dir.join(format!("cuda_stage_{}.mp4", i));

            match effect.as_str() {
                "color_grade" => {
                    cuda.apply_color_grading(&current, &temp_out, 0.85).await?;
                }
                "blur" => {
                    let radius = self.extract_param(intent, "blur", 5.0);
                    cuda.apply_blur(&current, &temp_out, radius).await?;
                }
                "denoise" => {
                    let strength = self.extract_param(intent, "denoise", 0.5);
                    cuda.apply_denoise(&current, &temp_out, strength).await?;
                }
                "sharpen" => {
                    let amount = self.extract_param(intent, "sharpen", 1.0);
                    cuda.apply_sharpen(&current, &temp_out, amount).await?;
                }
                custom => {
                    // Use AI to generate custom kernel
                    cuda.generate_custom_effect(&current, &temp_out, custom).await?;
                }
            }

            current = temp_out;
        }

        // Copy final result to output
        std::fs::copy(&current, output)?;
        std::fs::remove_dir_all(&temp_dir)?;

        Ok(output.to_path_buf())
    }

    fn parse_cuda_intent(&self, intent: &str) -> Vec<String> {
        let mut effects = Vec::new();

        if intent.contains("cinematic") || intent.contains("color") {
            effects.push("color_grade".to_string());
        }
        if intent.contains("blur") {
            effects.push("blur".to_string());
        }
        if intent.contains("denoise") || intent.contains("clean") {
            effects.push("denoise".to_string());
        }
        if intent.contains("sharp") {
            effects.push("sharpen".to_string());
        }

        // If no specific effects but creative intent, use AI
        if effects.is_empty() && (intent.contains("look") || intent.contains("style")) {
            effects.push(intent.to_string());
        }

        effects
    }

    fn extract_param(&self, intent: &str, param: &str, default: f32) -> f32 {
        // Simple parameter extraction
        // Could be enhanced with regex or LLM parsing
        if intent.contains("heavy") || intent.contains("strong") {
            return default * 1.5;
        }
        if intent.contains("light") || intent.contains("subtle") {
            return default * 0.5;
        }
        default
    }
}
```

### Example User Commands

```
User: "Make this video cinematic and remove all pauses"
Brain:
  1. Applies CUDA color grading (cinematic LUT)
  2. Passes to Motor Cortex for smart editing (pause removal)
  3. Returns polished result

User: "Clean up the noise and make it sharp"
Brain:
  1. Applies CUDA temporal denoising
  2. Applies CUDA unsharp mask
  3. Returns enhanced video

User: "Give this a dreamy, vintage film look"
Brain:
  1. Generates custom CUDA kernel via LLM
  2. Applies AI-generated effect
  3. Returns stylized video
```

### Neuroplasticity Integration

```rust
// In src/agent/neuroplasticity.rs

pub struct CudaKernelMemory {
    pub intent: String,
    pub kernel_name: String,
    pub params: HashMap<String, f32>,
    pub user_rating: f32,
    pub execution_time_ms: u64,
}

impl Neuroplasticity {
    pub fn learn_cuda_preference(&mut self, memory: CudaKernelMemory) {
        // Store successful CUDA kernel configurations
        self.cuda_history.push(memory);

        // Analyze patterns
        self.optimize_cuda_params();
    }

    fn optimize_cuda_params(&mut self) {
        // Find which parameters work best for each intent
        let mut intent_groups: HashMap<String, Vec<&CudaKernelMemory>> = HashMap::new();

        for mem in &self.cuda_history {
            intent_groups.entry(mem.intent.clone())
                .or_insert_with(Vec::new)
                .push(mem);
        }

        // For each intent, find optimal parameters
        for (intent, memories) in intent_groups {
            let best = memories.iter()
                .max_by(|a, b| a.user_rating.partial_cmp(&b.user_rating).unwrap());

            if let Some(best_config) = best {
                self.cuda_optimizations.insert(
                    intent.clone(),
                    best_config.params.clone()
                );
            }
        }
    }

    pub fn get_optimized_cuda_params(&self, intent: &str) -> Option<&HashMap<String, f32>> {
        self.cuda_optimizations.get(intent)
    }
}
```

### Motor Cortex Enhancement

```rust
// In src/agent/motor_cortex.rs

impl MotorCortex {
    pub async fn execute_smart_render_with_cuda(
        &mut self,
        intent: &str,
        input: &Path,
        output: &Path,
        visual_data: &[VisualScene],
        transcript: &[TranscriptSegment],
        audio_data: &AudioAnalysis,
        cuda_pipeline: Option<&CudaPipeline>,
    ) -> Result<String> {
        // 1. Pre-process with CUDA if needed
        let processed_input = if let Some(cuda) = cuda_pipeline {
            let temp = input.with_file_name("cuda_preprocessed.mp4");

            if intent.contains("enhance") || intent.contains("cinematic") {
                cuda.apply_color_grading(input, &temp, 0.8).await?;
                temp
            } else {
                input.to_path_buf()
            }
        } else {
            input.to_path_buf()
        };

        // 2. Execute standard smart render
        self.execute_smart_render(
            intent,
            &processed_input,
            output,
            visual_data,
            transcript,
            audio_data,
            false,
        ).await
    }
}
```

### GUI Integration Point

```rust
// In src/window.rs or dashboard

impl SynoidGui {
    fn render_cuda_controls(&mut self, ui: &mut egui::Ui) {
        ui.heading("🎨 CUDA Effects");

        ui.horizontal(|ui| {
            if ui.button("🎬 Cinematic").clicked() {
                self.queue_cuda_effect("cinematic color grading");
            }

            if ui.button("🌫️ Blur").clicked() {
                self.queue_cuda_effect("gaussian blur");
            }

            if ui.button("🧹 Denoise").clicked() {
                self.queue_cuda_effect("temporal denoise");
            }

            if ui.button("✨ Sharpen").clicked() {
                self.queue_cuda_effect("sharpen");
            }
        });

        ui.separator();

        ui.horizontal(|ui| {
            ui.label("Custom Effect:");
            ui.text_edit_singleline(&mut self.custom_cuda_intent);

            if ui.button("🤖 Generate").clicked() {
                self.queue_cuda_effect(&self.custom_cuda_intent.clone());
            }
        });
    }

    fn queue_cuda_effect(&mut self, intent: &str) {
        // Add to job queue
        let job = EditJob {
            effect_type: EffectType::CudaKernel,
            intent: intent.to_string(),
            // ... other fields
        };

        self.job_queue.push(job);
    }
}
```

## Summary

The CUDA-Agent integration is ready to use from:
- ✅ Brain (high-level orchestration)
- ✅ Motor Cortex (execution layer)
- ✅ Neuroplasticity (learning optimal parameters)
- ✅ GUI (user controls)
- ✅ CLI (direct command usage)

All components are designed to work together seamlessly with SYNOID's existing agentic architecture.
