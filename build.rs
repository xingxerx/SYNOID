use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=remotion-engine/package.json");

    // Do NOT run npm install automatically — it spawns a terminal window on
    // Windows and slows down every build. Instead, remind the developer to
    // install dependencies manually the first time.
    let engine_dir = Path::new("remotion-engine");
    if engine_dir.exists() && !engine_dir.join("node_modules").exists() {
        println!(
            "cargo:warning=⚠️  remotion-engine/node_modules not found.\
             \ncargo:warning=   Run `npm install` inside the `remotion-engine/` directory before rendering animations:\
             \ncargo:warning=       cd remotion-engine && npm install"
        );
    }
}
