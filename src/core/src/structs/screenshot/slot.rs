use std::path::PathBuf;
use std::time::Duration;

use super::request::ScreenshotRequest;
use super::slot_state::ScreenshotSlotState;

pub struct ScreenshotSlot {
    state: ScreenshotSlotState,
    next_ticket: u64,
}

impl Default for ScreenshotSlot {
    fn default() -> Self {
        Self::new()
    }
}

impl ScreenshotSlot {
    pub const TIMEOUT: Duration = Duration::from_secs(5);

    const TAKE_FAILED: &'static str = "the manager could not take the screenshot";
    const WRITE_FAILED: &'static str = "the manager could not write the screenshot";

    pub fn new() -> Self {
        Self { state: ScreenshotSlotState::Idle, next_ticket: 1 }
    }

    pub fn queue(&mut self, request: ScreenshotRequest, path: PathBuf) -> Result<u64, &'static str> {
        if self.state != ScreenshotSlotState::Idle {
            return Err("a screenshot is already in progress");
        }
        let ticket = self.next_ticket;
        self.next_ticket = self.next_ticket.wrapping_add(1);
        self.state = ScreenshotSlotState::Queued { ticket, request, path };
        Ok(ticket)
    }

    pub fn take_queued(&mut self) -> Option<(u64, ScreenshotRequest, PathBuf)> {
        match std::mem::replace(&mut self.state, ScreenshotSlotState::Idle) {
            ScreenshotSlotState::Queued { ticket, request, path } => {
                self.state = ScreenshotSlotState::Taken { ticket, path: path.clone() };
                Some((ticket, request, path))
            }
            other => {
                self.state = other;
                None
            }
        }
    }

    pub fn asked(&mut self, ticket: u64, sequence: u32) {
        if let ScreenshotSlotState::Taken { ticket: held, path } = &self.state
            && *held == ticket
        {
            self.state = ScreenshotSlotState::Asked { ticket, sequence, path: path.clone() };
        }
    }

    pub fn refused(&mut self, ticket: u64, why: &'static str) {
        let matches = matches!(
            &self.state,
            ScreenshotSlotState::Queued { ticket: held, .. }
                | ScreenshotSlotState::Taken { ticket: held, .. }
                | ScreenshotSlotState::Asked { ticket: held, .. }
                if *held == ticket
        );
        if matches {
            self.state = ScreenshotSlotState::Done { ticket, result: Err(why) };
        }
    }

    pub fn on_taken(&mut self, sequence: u32, ok: bool) {
        if ok {
            return;
        }
        if let ScreenshotSlotState::Asked { ticket, sequence: held, .. } = &self.state
            && *held == sequence
        {
            self.state = ScreenshotSlotState::Done { ticket: *ticket, result: Err(Self::TAKE_FAILED) };
        }
    }

    pub fn on_written(&mut self, sequence: u32, ok: bool) {
        if let ScreenshotSlotState::Asked { ticket, sequence: held, path } = &self.state
            && *held == sequence
        {
            let result = if ok { Ok(path.clone()) } else { Err(Self::WRITE_FAILED) };
            self.state = ScreenshotSlotState::Done { ticket: *ticket, result };
        }
    }

    pub fn outcome(&mut self, ticket: u64) -> Option<Result<PathBuf, &'static str>> {
        match std::mem::replace(&mut self.state, ScreenshotSlotState::Idle) {
            ScreenshotSlotState::Done { ticket: held, result } if held == ticket => Some(result),
            other => {
                self.state = other;
                None
            }
        }
    }

    pub fn abandon(&mut self, ticket: u64) {
        let held = match &self.state {
            ScreenshotSlotState::Idle => return,
            ScreenshotSlotState::Queued { ticket, .. }
            | ScreenshotSlotState::Taken { ticket, .. }
            | ScreenshotSlotState::Asked { ticket, .. }
            | ScreenshotSlotState::Done { ticket, .. } => *ticket,
        };
        if held == ticket {
            self.state = ScreenshotSlotState::Idle;
        }
    }
}
