fn main() {
    println!("{:?}", std::process::Command::new("yt-dlp").args(&["--version"]).output());
}
