use std::sync::Arc;

use crate::structs::JoinRequest;

// Where a join step is: leaving the world it started in, or joining its server.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum JoinPhase {
    // the step started in a world; it asks to leave until the title shows
    Leaving,
    // it asks to join until the world shows; the request is made once for the phase
    Joining(Arc<JoinRequest>),
}
