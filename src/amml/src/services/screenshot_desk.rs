use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use amm_core::structs::event::ModEvent;
use headless_input_core::structs::{ScreenshotRequest, ScreenshotSlot};
use headless_input_mcp::port::PortError;

use crate::traits::ScreenshotPort;

// what the desk keeps beside the slot, under the same mutex: whether it is closed, the ticket the port thread waits
// for, and the `Overlay` lease of the shot in flight. The lease is taken and given back on the game thread only:
// `held` is the manager sequence of the shot that holds it, and `release_due` says the shot ended, so the next pump
// gives the lease back
struct Desk {
    slot: ScreenshotSlot,
    closed: bool,
    waiting: Option<u64>,
    held: Option<u32>,
    release_due: bool,
}

// where the port thread hands a screenshot request to the game thread and waits for the manager's answer. The port
// thread queues the request and waits on the condvar for at most `timeout`; the game thread only takes the request,
// asks the manager, records events and notifies, so it never waits. One request is in flight at a time
pub struct ScreenshotDesk {
    desk: Mutex<Desk>,
    answered: Condvar,
    timeout: Duration,
    // whether the manager takes screenshots, as the link saw it at attach; true until told otherwise
    supported: AtomicBool,
}

impl ScreenshotDesk {
    const UNSUPPORTED: &'static str = "the mod manager does not take screenshots";
    const NOT_TAKEN: &'static str = "the mod manager did not accept the screenshot request";
    const NOT_LINKED: &'static str = "the mod manager is not linked";
    const NO_ANSWER: &'static str = "the manager did not answer the screenshot";
    const CLOSED: &'static str = "the mod is disabled";
    const NO_LEASE: &'static str = "the manager refused the Overlay lease";

    pub fn new(timeout: Duration) -> Self {
        Self {
            desk: Mutex::new(Desk {
                slot: ScreenshotSlot::new(),
                closed: false,
                waiting: None,
                held: None,
                release_due: false,
            }),
            answered: Condvar::new(),
            timeout,
            supported: AtomicBool::new(true),
        }
    }

    pub fn new_shared(timeout: Duration) -> Arc<Self> {
        Arc::new(Self::new(timeout))
    }

    // the manager's screenshot support, read when the link attaches
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
                desk.release_due = true;
                return Err(PortError::Timeout(Self::NO_ANSWER.to_string()));
            };
            desk = self.answered.wait_timeout(desk, left).unwrap_or_else(PoisonError::into_inner).0;
        }
    }

    // the mod is disabled: the waiting caller is refused at once and later calls are refused until `open`. The
    // lease, if held, goes back on the next pump
    pub fn close(&self) {
        let mut desk = self.lock();
        desk.closed = true;
        desk.release_due = true;
        if let Some(ticket) = desk.waiting {
            desk.slot.refused(ticket, Self::CLOSED);
        }
        drop(desk);
        self.answered.notify_all();
    }

    pub fn open(&self) {
        self.lock().closed = false;
    }

    // game thread, once per tick: gives back the lease of a shot that ended, then sends a queued request to the
    // manager under the `Overlay` lease
    pub fn pump(&self, port: &impl ScreenshotPort) {
        let mut desk = self.lock();
        if desk.release_due {
            desk.release_due = false;
            if desk.held.take().is_some() {
                port.release_overlay();
            }
        }
        let Some((ticket, request, path)) = desk.slot.take_queued() else { return };
        if !port.supports_screenshots() {
            desk.slot.refused(ticket, Self::UNSUPPORTED);
        } else if !port.acquire_overlay() {
            desk.slot.refused(ticket, Self::NO_LEASE);
        } else {
            match port.request_screenshot(&path, request.layer) {
                Ok(sequence) => {
                    desk.held = Some(sequence);
                    desk.slot.asked(ticket, sequence);
                }
                Err(error) => {
                    curia::warn!("the manager refused the screenshot request", { error: error.to_string() });
                    port.release_overlay();
                    desk.slot.refused(ticket, Self::NOT_TAKEN);
                }
            }
        }
        drop(desk);
        self.answered.notify_all();
    }

    // game thread: a screenshot answer of the manager; the lease of the shot it ends goes back on the next pump
    pub fn on_event(&self, event: &ModEvent) {
        let mut desk = self.lock();
        let ended = match *event {
            ModEvent::ScreenshotTaken { sequence, ok } => {
                desk.slot.on_taken(sequence, ok);
                (!ok).then_some(sequence)
            }
            ModEvent::ScreenshotWritten { sequence, ok } => {
                desk.slot.on_written(sequence, ok);
                Some(sequence)
            }
            _ => return,
        };
        if ended.is_some() && desk.held == ended {
            desk.release_due = true;
        }
        drop(desk);
        self.answered.notify_all();
    }

    fn lock(&self) -> MutexGuard<'_, Desk> {
        self.desk.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
