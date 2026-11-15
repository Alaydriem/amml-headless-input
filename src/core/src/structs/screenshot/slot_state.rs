use std::path::PathBuf;

use super::request::ScreenshotRequest;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScreenshotSlotState {
    Idle,
    Queued { ticket: u64, request: ScreenshotRequest, path: PathBuf },
    // the game thread took the request and has not asked the manager yet
    Taken { ticket: u64, path: PathBuf },
    Asked { ticket: u64, sequence: u32, path: PathBuf },
    Done { ticket: u64, result: Result<PathBuf, &'static str> },
}
