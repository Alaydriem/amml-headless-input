use std::borrow::Cow;

// What the codes the game passes to a join's result callback mean. Only the codes seen on the game's failure paths
// are failures; any other code, 0 included, is not one.
pub struct JoinResultCodes;

impl JoinResultCodes {
    // the join failed because the server id was not found
    pub const NOT_FOUND: i32 = 1;
    // the host name did not resolve
    pub const RESOLVE_FAILED: i32 = 8;
    // the connectivity test failed
    pub const CONNECTIVITY_FAILED: i32 = 9;

    // the reason a join ended with `code`, or None when the code is not a failure
    pub fn failure(code: i32) -> Option<Cow<'static, str>> {
        let why = match code {
            Self::NOT_FOUND => format!("the join failed: the game did not find the server (result {code})"),
            Self::RESOLVE_FAILED => format!("the join failed: the game could not resolve the host (result {code})"),
            Self::CONNECTIVITY_FAILED => format!("the join failed: the connectivity test failed (result {code})"),
            5..=7 | 0x15 => format!("the join failed: the game refused it (result {code})"),
            _ => return None,
        };
        Some(Cow::Owned(why))
    }
}
