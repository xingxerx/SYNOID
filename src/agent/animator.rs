use std::path::{Path, PathBuf};
use tokio::process::Command;
use tracing::{info, warn};

pub struct Animator {
    pub engine_dir: PathBuf,
}

impl Animator {
    pub fn new(root_dir: &Path) -> Self {
        Self {
            engine_dir: root_dir.join("remotion-engine"),
        }
    }

    /// Check if the remotion engine is initialized
    pub async fn is_initialized(&self) -> bool {
        self.engine_dir.join("package.json").exists()
            && self.engine_dir.join("node_modules").exists()
    }

    /// Renders a composition via Remotion (DynamicAnimation by default)
    pub async fn render_animation(
        &self,
        composition_name: &str,
        payload_json_path: &Path,
        output_video: &Path,
    ) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
        // Default to DynamicAnimation if no composition specified
        let comp = if composition_name.is_empty() { "DynamicAnimation" } else { composition_name };
        info!("[ANIMATOR] Rendering composition '{}' via Remotion...", comp);

        if !self.is_initialized().await {
            return Err("Remotion engine is not initialized. Run npm install in remotion-engine/.".into());
        }

        // npx remotion render src/index.tsx <CompositionName> <Output> --props <Payload>
        let mut cmd = if cfg!(windows) {
            Command::new("cmd")
        } else {
            Command::new("sh")
        };
        
        let output = if cfg!(windows) {
            cmd.arg("/C").arg(format!("npx remotion render src/index.tsx {} {} --props {}", comp, output_video.to_string_lossy(), payload_json_path.to_string_lossy()))
        } else {
            cmd.arg("-c").arg(format!("npx remotion render src/index.tsx \"{}\" \"{}\" --props \"{}\"", comp, output_video.to_string_lossy(), payload_json_path.to_string_lossy()))
        }
        .current_dir(&self.engine_dir)
        .output()
        .await?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            warn!("[ANIMATOR] Remotion render failed: {}", err);
            return Err(format!("Remotion error: {}", err).into());
        }

        info!("[ANIMATOR] Animation rendered successfully: {:?}", output_video);
        Ok(output_video.to_path_buf())
    }
}
