use std::sync::{Arc, Mutex, PoisonError};

use amml_api::{JoinOutcome, StartWorldOutcome};
use headless_input_core::run::{JoinResultCodes, WorldStartCodes};

// The loader's join and world start answers as the result codes the core reads, and the slot the `JoinServerResult`
// listener fills for the join service. The slot holds the code of the last answer until it is taken or cleared
#[derive(Clone, Default)]
pub struct JoinCodes {
    answer: Arc<Mutex<Option<i32>>>,
}

impl JoinCodes {
    // the code the core gives the join refusal or none for `outcome`
    pub fn join_code(outcome: JoinOutcome) -> i32 {
        match outcome {
            JoinOutcome::NotFound => JoinResultCodes::NOT_FOUND,
            JoinOutcome::ResolveFailed => JoinResultCodes::RESOLVE_FAILED,
            JoinOutcome::ConnectivityFailed => JoinResultCodes::CONNECTIVITY_FAILED,
            JoinOutcome::Refused => Self::REFUSED,
            _ => 0,
        }
    }

    // the code the core reads for a world start answer
    pub fn start_code(outcome: StartWorldOutcome) -> i32 {
        match outcome {
            StartWorldOutcome::Started => WorldStartCodes::OK,
            StartWorldOutcome::NotInLevelList => WorldStartCodes::NOT_IN_LEVEL_LIST,
            StartWorldOutcome::StartupInProgress => WorldStartCodes::STARTUP_IN_PROGRESS,
            StartWorldOutcome::SavedInNewerVersion => WorldStartCodes::SAVED_IN_NEWER_VERSION,
            StartWorldOutcome::PlatformLockedContent => WorldStartCodes::PLATFORM_LOCKED_CONTENT,
            StartWorldOutcome::NotFoundInFileSystem => WorldStartCodes::NOT_FOUND_IN_FILE_SYSTEM,
            StartWorldOutcome::SlimCacheData => WorldStartCodes::SLIM_CACHE_DATA,
            _ => Self::OTHER_REFUSAL,
        }
    }

    // the game's answer to the join, from the `JoinServerResult` listener
    pub fn answer(&self, outcome: JoinOutcome) {
        curia::info!("the game answered the join", { outcome: outcome.name() });
        *self.answer.lock().unwrap_or_else(PoisonError::into_inner) = Some(Self::join_code(outcome));
    }

    // the code of the last answer, taken once
    pub fn take(&self) -> Option<i32> {
        self.answer.lock().unwrap_or_else(PoisonError::into_inner).take()
    }

    pub fn clear(&self) {
        *self.answer.lock().unwrap_or_else(PoisonError::into_inner) = None;
    }

    // a refusal code the core counts as a refusal
    const REFUSED: i32 = 5;
    // a world start refusal of no known kind
    const OTHER_REFUSAL: i32 = 99;
}
