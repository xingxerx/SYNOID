# Sentinel Journal

## 2026-10-24 - External Command Argument Injection

**Vulnerability:** External commands (`ffprobe`, `ffmpeg`, `yt-dlp`) were invoked with user-controlled paths as direct arguments. If a path started with `-`, it could be interpreted as a flag, leading to denial of service or potentially worse. Additionally, `path.to_str().unwrap()` was used, which panics on invalid UTF-8 paths.

**Learning:** The `std::process::Command` (and `tokio` equivalent) interface does not automatically protect against argument injection if the arguments are flags. While it escapes arguments for shell execution, it does not prevent the target binary from parsing them as flags.
Crucially, **`ffmpeg` and `ffprobe` do NOT support the standard `--` delimiter** for stopping option parsing in all contexts (specifically for output files, it might fail). The robust solution is to ensure paths are either absolute or explicitly relative (prefixed with `./`).

**Prevention:** Ensure all file paths passed to external commands are safe from flag interpretation. If a path is relative, prepend `./`. Use `Path::to_string_lossy()` or pass `Path` directly (via `arg()`) instead of `unwrap()` to handle invalid UTF-8 paths gracefully.

## 2026-10-25 - Command Argument Fragmentation (Space Injection)

**Vulnerability:** The internal `execute_one_shot_render` function constructed an FFmpeg command as a single formatted `String` and returned it. The consumer (`main.rs`) then naively split this string using `split_whitespace()`. This caused filenames containing spaces (e.g., "My Video.mp4") to be fragmented into multiple arguments ("My", "Video.mp4"), leading to command execution failure or potentially allowing argument injection if not properly sanitized.

**Learning:** Returning a raw command string from a helper function forces the caller to re-parse it, which is error-prone. Spaces in arguments are significant and must be preserved. `split_whitespace()` is destructive for shell commands that rely on quoting or escaping.

**Prevention:** Helper functions that generate commands should return `Vec<String>` (a list of arguments) or a `Command` builder object, never a raw `String` intended for shell execution (unless using `sh -c`). This preserves the integrity of individual arguments, regardless of spaces or special characters.

## 2026-10-26 - Arbitrary File Read (Path Traversal) in API

**Vulnerability:** The `/api/stream` endpoint in `src/server.rs` served files using `tower_http::services::ServeFile` with paths provided directly by user query parameters. This allowed traversal (`..`) to arbitrary files on the system (e.g., `/etc/passwd`), exposing sensitive data to any client, including malicious websites via CORS.

**Learning:** `tower_http::services::ServeFile` does NOT automatically sandbox file access to the current directory or a specific root unless used with `ServeDir`. When constructing `ServeFile` from a user-provided path, manual validation is mandatory to prevent traversal and enforce expected file types.

**Prevention:** Always validate user-provided paths before passing them to file serving utilities. Reject paths containing `ParentDir` (`..`) components and enforce a whitelist of allowed file extensions (e.g., `.mp4`, `.mkv`) to restrict access to intended media only.
