use std::process::Command;
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=remotion-engine/package.json");
    
    let engine_dir = Path::new("remotion-engine");
    if engine_dir.exists() {
        // Run npm install in the remotion-engine directory
        let status = if cfg!(target_os = "windows") {
            Command::new("cmd")
                .arg("/C")
                .arg("npm install")
                .current_dir(&engine_dir)
                .status()
        } else {
            Command::new("sh")
                .arg("-c")
                .arg("npm install")
                .current_dir(&engine_dir)
                .status()
        };

        if let Ok(status) = status {
            if !status.success() {
                println!("cargo:warning=npm install failed in remotion-engine");
            }
        } else {
            println!("cargo:warning=Failed to execute npm install in remotion-engine");
        }
    }
}
