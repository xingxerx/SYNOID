## 2026-05-22 - [Argument Injection in External Command]
**Vulnerability:** Argument Injection in `yt-dlp` invocation via `url` parameter.
**Learning:** External CLIs often interpret arguments starting with `-` as flags, even in positional contexts. `Command::args` in Rust prevents shell injection but not argument injection for the target program.
**Prevention:** Always use `--` to delimit options from positional arguments when invoking external commands like `yt-dlp`, `git`, etc.
