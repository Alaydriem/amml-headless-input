use crate::control::Step;
use crate::run::LiveAction;

// The player reads and the game calls the game build gives. A step that needs one the build does not give is refused
// when its control file loads
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Capabilities {
    pub position: bool,
    // the yaw and pitch, read and written
    pub rotation: bool,
    pub dimension: bool,
    // the game's join by address
    pub join: bool,
    // the game's leave of the world
    pub leave: bool,
    // sending a slash command to the server as the local player
    pub command: bool,
}

impl Capabilities {
    pub fn new(position: bool, rotation: bool, dimension: bool, join: bool, leave: bool, command: bool) -> Self {
        Self { position, rotation, dimension, join, leave, command }
    }

    pub fn all() -> Self {
        Self::new(true, true, true, true, true, true)
    }

    // the first read or call `step` needs that is missing; None when the step can run. A repeat is checked through
    // its own steps. A join that starts in a world leaves it first, so it needs the leave too
    pub fn missing_for(&self, step: &Step) -> Option<&'static str> {
        match step {
            Step::WalkTo { .. } if !self.position => Some("the player position"),
            Step::WalkTo { .. } | Step::Turn { .. } | Step::LookAt { .. } if !self.rotation => {
                Some("the player rotation")
            }
            Step::WalkUntilDimension { .. } if !self.dimension => Some("the dimension"),
            Step::JoinServer { .. } if !self.join => Some("the server join"),
            Step::JoinServer { .. } | Step::LeaveWorld { .. } if !self.leave => Some("the world leave"),
            Step::Command { .. } if !self.command => Some("sending a command"),
            _ => None,
        }
    }

    // the first read or call a live action needs that is missing; None when it can run. A live join that starts in
    // a world leaves it first, so it needs the leave too
    pub fn missing_for_live(&self, action: &LiveAction) -> Option<&'static str> {
        match action {
            LiveAction::Turn { .. } | LiveAction::LookAt { .. } if !self.rotation => Some("the player rotation"),
            LiveAction::JoinServer { .. } if !self.join => Some("the server join"),
            LiveAction::JoinServer { .. } | LiveAction::LeaveWorld { .. } if !self.leave => Some("the world leave"),
            _ => None,
        }
    }
}
