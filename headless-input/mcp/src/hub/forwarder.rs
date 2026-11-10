use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use tokio::sync::oneshot;

use super::{InstanceId, MemberRegistry, PendingGuard};
use crate::dto::{Forwarded, MemberReply, MemberRequest};
use crate::port::PortError;

// Sends a command to a named member and awaits its reply. At most `PENDING_CAPACITY` commands wait for a reply at
// once; one past it is refused as busy. A command leaves the pending map when its reply comes or its time-out ends.
#[derive(Debug)]
pub struct Forwarder {
    registry: Arc<MemberRegistry>,
    pending: Mutex<HashMap<u64, oneshot::Sender<MemberReply>>>,
    next: AtomicU64,
    timeout: Duration,
}

impl Forwarder {
    pub const TIMEOUT: Duration = Duration::from_secs(10);
    pub const PENDING_CAPACITY: usize = 256;

    pub fn new(registry: Arc<MemberRegistry>, timeout: Duration) -> Self {
        Self { registry, pending: Mutex::new(HashMap::new()), next: AtomicU64::new(1), timeout }
    }

    pub fn new_shared(registry: Arc<MemberRegistry>, timeout: Duration) -> Arc<Self> {
        Arc::new(Self::new(registry, timeout))
    }

    pub fn registry(&self) -> &Arc<MemberRegistry> {
        &self.registry
    }

    // queues `request` for member `id` and waits for the reply, at most the time-out
    pub async fn send(&self, id: &InstanceId, request: MemberRequest) -> Result<MemberReply, PortError> {
        let command = self.next.fetch_add(1, Ordering::Relaxed);
        let (sender, receiver) = oneshot::channel();
        {
            let mut pending = self.lock();
            if pending.len() >= Self::PENDING_CAPACITY {
                return Err(PortError::Busy);
            }
            pending.insert(command, sender);
        }
        let _guard = PendingGuard::new(self, command);
        self.registry.submit(id, Forwarded { command, request })?;
        match tokio::time::timeout(self.timeout, receiver).await {
            Ok(Ok(reply)) => Ok(reply),
            Ok(Err(_)) | Err(_) => {
                Err(PortError::Timeout(format!("instance {id} did not answer in {} ms", self.timeout.as_millis())))
            }
        }
    }

    // hands the reply to the waiting `send`; false when no command `command` waits (it timed out)
    pub fn complete(&self, command: u64, reply: MemberReply) -> bool {
        match self.lock().remove(&command) {
            Some(sender) => sender.send(reply).is_ok(),
            None => false,
        }
    }

    // the number of commands that wait for a reply
    pub fn pending(&self) -> usize {
        self.lock().len()
    }

    pub(crate) fn forget(&self, command: u64) {
        self.lock().remove(&command);
    }

    fn lock(&self) -> MutexGuard<'_, HashMap<u64, oneshot::Sender<MemberReply>>> {
        self.pending.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}
