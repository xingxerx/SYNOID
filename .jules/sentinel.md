# Sentinel Journal

## 2026-10-24 - External Command Argument Injection

**Vulnerability:** External commands (`ffprobe`, `ffmpeg`, `yt-dlp`) were invoked with user-controlled paths as direct arguments. If a path started with `-`, it could be interpreted as a flag, leading to denial of service or potentially worse. Additionally, `path.to_str().unwrap()` was used, which panics on invalid UTF-8 paths.

**Learning:** The `std::process::Command` (and `tokio` equivalent) interface does not automatically protect against argument injection if the arguments are flags. While it escapes arguments for shell execution, it does not prevent the target binary from parsing them as flags.
Crucially, **`ffmpeg` and `ffprobe` do NOT support the standard `--` delimiter** for stopping option parsing in all contexts (specifically for output files, it might fail). The robust solution is to ensure paths are either absolute or explicitly relative (prefixed with `./`).

**Prevention:** Ensure all file paths passed to external commands are safe from flag interpretation. If a path is relative, prepend `./`. Use `Path::to_string_lossy()` or pass `Path` directly (via `arg()`) instead of `unwrap()` to handle invalid UTF-8 paths gracefully.
