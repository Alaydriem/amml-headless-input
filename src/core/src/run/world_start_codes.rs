use std::borrow::Cow;

// What the codes the game's local world start returns mean. The call answers at once, with no callback. Code 0 means the
// start was taken, and code 3 means the game is not ready to start a world yet; every other code is a failure.
pub struct WorldStartCodes;

impl WorldStartCodes {
    // the start was requested
    pub const OK: i32 = 0;
    // the world is not in the game's world list
    pub const NOT_IN_LEVEL_LIST: i32 = 1;
    // another start has not finished
    pub const STARTUP_IN_PROGRESS: i32 = 2;
    // the game server starter is not ready, as when the old server still shuts down
    pub const NOT_READY: i32 = 3;
    // the world was saved by a newer version of the game
    pub const SAVED_IN_NEWER_VERSION: i32 = 9;
    // the world has platform locked content the player did not confirm
    pub const PLATFORM_LOCKED_CONTENT: i32 = 12;
    // the world folder is not on disk
    pub const NOT_FOUND_IN_FILE_SYSTEM: i32 = 13;
    // the world has slim cache data
    pub const SLIM_CACHE_DATA: i32 = 14;

    // whether the game is not ready: not a failure, the start is asked again on a later frame
    pub fn is_not_ready(code: i32) -> bool {
        code == Self::NOT_READY
    }

    // whether `code` is a failure; no allocation
    pub fn is_failure(code: i32) -> bool {
        !matches!(code, Self::OK | Self::NOT_READY)
    }

    // the reason a world start ended with `code`, or None when the code is not a failure
    pub fn failure(code: i32) -> Option<Cow<'static, str>> {
        let reason = match code {
            Self::OK | Self::NOT_READY => return None,
            Self::NOT_IN_LEVEL_LIST => "the world is not in the game's world list",
            Self::STARTUP_IN_PROGRESS => "another world start is in progress",
            Self::SAVED_IN_NEWER_VERSION => "the world was saved in a newer version of the game",
            Self::PLATFORM_LOCKED_CONTENT => "the world has platform locked content that is not confirmed",
            Self::NOT_FOUND_IN_FILE_SYSTEM => "the world was not found in the file system",
            Self::SLIM_CACHE_DATA => "the world has slim cache data",
            _ => "the game refused it",
        };
        Some(Cow::Owned(format!("the world join failed: {reason} (result {code})")))
    }
}
