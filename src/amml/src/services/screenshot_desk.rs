use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use amm_core::structs::event::ModEvent;
use amml_api::ShotId;
use headless_input_core::structs::{ScreenshotRequest, ScreenshotSlot};
use headless_input_mcp::port::PortError;

use crate::traits::ScreenshotPort;

// what the desk keeps beside the slot, under the same mutex: whether it is closed and the ticket the port thread waits
// for
struct Desk {
    slot: ScreenshotSlot,
    closed: bool,
    waiting: Option<u64>,
}

// where the port thread hands a screenshot request to the game thread and waits for the manager's answer. The port
// thread queues the request and waits on the condvar for at most `timeout`; the game thread only takes the request,
// asks the manager, records events and notifies, so it never waits. One request is in flight at a time
pub struct ScreenshotDesk {
    desk: Mutex<Desk>,
    answered: Condvar,
    timeout: Duration,
    // whether the loader takes screenshots (area `overlay.screenshot`); true until told otherwise
    supported: AtomicBool,
}

impl ScreenshotDesk {
    const UNSUPPORTED: &'static str = "the loader does not take screenshots on this game";
    const NOT_TAKEN: &'static str = "the loader did not accept the screenshot request";
    const NOT_LINKED: &'static str = "the mod manager is not linked";
    const NO_ANSWER: &'static str = "the loader did not answer the screenshot";
    const CLOSED: &'static str = "the mod is disabled";

    pub fn new(timeout: Duration) -> Self {
        Self {
            desk: Mutex::new(Desk { slot: ScreenshotSlot::new(), closed: false, waiting: None }),
            answered: Condvar::new(),
            timeout,
            supported: AtomicBool::new(true),
        }
    }

    pub fn new_shared(timeout: Duration) -> Arc<Self> {
        Arc::new(Self::new(timeout))
    }

    // the loader's screenshot support, read at enable
    pub fn set_supported(&self, supported: bool) {
        self.supported.store(supported, Ordering::Relaxed);
    }

    // port thread: queues the request and waits for the answer; the path of the written PNG
    pub fn take(&self, request: ScreenshotRequest, path: PathBuf, linked: bool) -> Result<PathBuf, PortError> {
        if !linked {
            return Err(PortError::Refused(Self::NOT_LINKED.to_string()));
        }
        if !self.supported.load(Ordering::Relaxed) {
            return Err(PortError::Refused(Self::UNSUPPORTED.to_string()));
        }
        let deadline = Instant::now() + self.timeout;
        let mut desk = self.lock();
        if desk.closed {
            return Err(PortError::Refused(Self::CLOSED.to_string()));
        }
        let ticket = desk.slot.queue(request, path).map_err(|why| PortError::Refused(why.to_string()))?;
        desk.waiting = Some(ticket);
        loop {
            if let Some(outcome) = desk.slot.outcome(ticket) {
                desk.waiting = None;
                return outcome.map_err(|why| PortError::Refused(why.to_string()));
            }
            let Some(left) = deadline.checked_duration_since(Instant::now()).filter(|left| !left.is_zero()) else {
                desk.slot.abandon(ticket);
                desk.waiting = None;
                return Err(PortError::Timeout(Self::NO_ANSWER.to_string()));
            };
            desk = self.answered.wait_timeout(desk, left).unwrap_or_else(PoisonError::into_inner).0;
        }
    }

    // the mod is disabled: the waiting caller is refused at once and later calls are refused until `open`
    pub fn close(&self) {
        let mut desk = self.lock();
        desk.closed = true;
        if let Some(ticket) = desk.waiting {
            desk.slot.refused(ticket, Self::CLOSED);
        }
        drop(desk);
        self.answered.notify_all();
    }

    pub fn open(&self) {
        self.lock().closed = false;
    }

    // game thread, once per frame: sends a queued request to the loader
    pub fn pump(&self, port: &impl ScreenshotPort) {
        let mut desk = self.lock();
        let Some((ticket, request, path)) = desk.slot.take_queued() else { return };
        if !port.supports_screenshots() {
            desk.slot.refused(ticket, Self::UNSUPPORTED);
        } else {
            match port.request_screenshot(&path, request.layer) {
                Ok(sequence) => desk.slot.asked(ticket, sequence),
                Err(error) => {
                    curia::warn!("the loader refused the screenshot request", { error: error.to_string() });
                    desk.slot.refused(ticket, Self::NOT_TAKEN);
                }
            }
        }
        drop(desk);
        self.answered.notify_all();
    }

    // game thread: a screenshot answer of the loader, passed on by the link
    pub fn on_event(&self, event: &ModEvent) {
        let mut desk = self.lock();
        match *event {
            ModEvent::ScreenshotTaken { id, ok } => desk.slot.on_taken(Self::sequence(id), ok),
            ModEvent::ScreenshotWritten { id, ok } => {
                if ok {
                    curia::info!("screenshot written", { id: id.0 });
                }
                desk.slot.on_written(Self::sequence(id), ok);
            }
            _ => return,
        }
        drop(desk);
        self.answered.notify_all();
    }

    // the slot's sequence of a loader shot id; the loader counts its ids from 1, so the low 32 bits name one shot
    pub fn sequence(id: ShotId) -> u32 {
        id.0 as u32
    }

    fn lock(&self) -> MutexGuard<'_, Desk> {
        self.desk.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
