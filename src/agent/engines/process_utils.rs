// Process Utilities for SYNOID - Stealth Execution
// Copyright (c) 2026 xingxerx_The_Creator | SYNOID
//
// Provides helpers to spawn child processes without popping console windows on Windows.

use std::process::Command as StdCommand;
use tokio::process::Command as TokioCommand;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

/// Extension trait for creating "stealth" processes that don't show terminal windows on Windows.
pub trait CommandExt {
    fn stealth(&mut self) -> &mut Self;
}

impl CommandExt for StdCommand {
    fn stealth(&mut self) -> &mut Self {
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            self.creation_flags(CREATE_NO_WINDOW);
        }
        self
    }
}

impl CommandExt for TokioCommand {
    fn stealth(&mut self) -> &mut Self {
        #[cfg(windows)]
        {
            self.creation_flags(CREATE_NO_WINDOW);
        }
        self
    }
}
