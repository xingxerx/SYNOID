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
    }

    /// Renders a lower third or animation via Remotion
    pub async fn render_animation(
        &self,
        composition_name: &str,
        payload_json_path: &Path,
        output_video: &Path,
    ) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
        info!("[ANIMATOR] Rendering composition '{}' via Remotion...", composition_name);

        if !self.is_initialized().await {
            return Err("Remotion engine is not initialized. Please run npm install in remotion-engine.".into());
        }

        // Running Remotion requires local node_modules
        // npx remotion render src/index.ts <CompositionName> <Output> --props <Payload>
        let output = Command::new("npx")
            .current_dir(&self.engine_dir)
            .arg("remotion")
            .arg("render")
            .arg("src/index.ts")
            .arg(composition_name)
            .arg(output_video)
            .arg("--props")
            .arg(payload_json_path)
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
