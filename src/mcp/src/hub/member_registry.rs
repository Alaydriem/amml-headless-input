use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use tokio::sync::mpsc::error::TrySendError;

use super::member_entry::MemberEntry;
use super::{InstanceId, MemberInfo, PollGuard};
use crate::dto::Forwarded;
use crate::errors::RegistryError;
use crate::port::PortError;

// The instances the hub serves, the hub itself among them. At most `CAPACITY` members: a new id past it is refused,
// nothing is evicted to make room. A member leaves when it unregisters, when its poll connection closes, or when it
// is not heard from for `STALE`. Each member has a command queue of `QUEUE`; a command past it is refused as busy.
#[derive(Debug, Default)]
pub struct MemberRegistry {
    members: Mutex<HashMap<InstanceId, MemberEntry>>,
    generation: AtomicU64,
}

impl MemberRegistry {
    pub const CAPACITY: usize = 64;
    pub const QUEUE: usize = 16;
    // a member is dropped when its heartbeats (every second) and polls stop for this long
    pub const STALE: Duration = Duration::from_secs(5);

    pub fn new() -> Self {
        Self::default()
    }

    pub fn new_shared() -> Arc<Self> {
        Arc::new(Self::new())
    }

    // adds the member, or replaces a remote member of the same id and process (it reconnected). Refused past
    // `CAPACITY`, for the id of the hub's own instance, and for the id of a remote member of another process (two
    // games started from one instance folder); a member that stops is pruned after `STALE`, which frees its id
    pub fn register(&self, info: MemberInfo) -> Result<(), RegistryError> {
        let mut members = self.lock();
        match members.get(&info.id) {
            Some(existing) if !info.local && (existing.info.local || existing.info.pid != info.pid) => {
                return Err(RegistryError::Taken(info.id.to_string()));
            }
            Some(_) => {}
            None if members.len() >= Self::CAPACITY => return Err(RegistryError::Full(Self::CAPACITY)),
            None => {}
        }
        let generation = self.generation.fetch_add(1, Ordering::Relaxed) + 1;
        members.insert(info.id.clone(), MemberEntry::new(info, Self::QUEUE, generation));
        Ok(())
    }

    // removes a remote member; the hub's own instance stays
    pub fn unregister(&self, id: &InstanceId) -> bool {
        let mut members = self.lock();
        if members.get(id).is_some_and(|entry| !entry.info.local) {
            members.remove(id);
            return true;
        }
        false
    }

    // notes that the member was heard from; false when it is not registered
    pub fn touch(&self, id: &InstanceId) -> bool {
        match self.lock().get_mut(id) {
            Some(entry) => {
                entry.seen = Instant::now();
                true
            }
            None => false,
        }
    }

    pub fn get(&self, id: &InstanceId) -> Option<MemberInfo> {
        self.lock().get(id).map(|entry| entry.info.clone())
    }

    // every member, sorted by id
    pub fn list(&self) -> Vec<MemberInfo> {
        let mut list: Vec<MemberInfo> = self.lock().values().map(|entry| entry.info.clone()).collect();
        list.sort_by(|left, right| left.id.cmp(&right.id));
        list
    }

    pub fn ids(&self) -> Vec<String> {
        self.list().into_iter().map(|info| info.id.to_string()).collect()
    }

    pub fn len(&self) -> usize {
        self.lock().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    // queues a command for the member; `Busy` when its queue is full
    pub fn submit(&self, id: &InstanceId, forwarded: Forwarded) -> Result<(), PortError> {
        let sender = match self.lock().get(id) {
            Some(entry) => entry.sender.clone(),
            None => return Err(PortError::UnknownInstance(id.to_string())),
        };
        sender.try_send(forwarded).map_err(|error| match error {
            TrySendError::Full(_) => PortError::Busy,
            TrySendError::Closed(_) => PortError::UnknownInstance(id.to_string()),
        })
    }

    // the next command for the member, waiting up to `wait`; None when none came. A poll dropped before it ends
    // drops the member: its connection closed
    pub async fn next(self: &Arc<Self>, id: &InstanceId, wait: Duration) -> Result<Option<Forwarded>, PortError> {
        let (generation, receiver) = {
            let mut members = self.lock();
            let Some(entry) = members.get_mut(id) else {
                return Err(PortError::UnknownInstance(id.to_string()));
            };
            entry.seen = Instant::now();
            entry.polls += 1;
            (entry.generation, Arc::clone(&entry.receiver))
        };
        let mut guard = PollGuard::new(Arc::clone(self), id.clone(), generation);
        let taken = tokio::time::timeout(wait, async {
            let mut queue = receiver.lock().await;
            queue.recv().await
        })
        .await;
        guard.finish();
        self.touch(id);
        Ok(taken.ok().flatten())
    }

    // the ids of the members with a poll open now, sorted
    pub fn polling(&self) -> Vec<String> {
        let mut ids: Vec<String> =
            self.lock().values().filter(|entry| entry.polls > 0).map(|entry| entry.info.id.to_string()).collect();
        ids.sort();
        ids
    }

    pub(crate) fn poll_ended(&self, id: &InstanceId, generation: u64) {
        if let Some(entry) = self.lock().get_mut(id).filter(|entry| entry.generation == generation) {
            entry.polls = entry.polls.saturating_sub(1);
        }
    }

    // drops a remote member whose poll connection closed, unless it registered again since
    pub fn connection_closed(&self, id: &InstanceId, generation: u64) {
        let mut members = self.lock();
        if members.get(id).is_some_and(|entry| entry.generation == generation && !entry.info.local) {
            members.remove(id);
        }
    }

    // drops the remote members not heard from for `stale`; returns their ids
    pub fn prune(&self, stale: Duration) -> Vec<InstanceId> {
        let mut members = self.lock();
        let gone: Vec<InstanceId> = members
            .values()
            .filter(|entry| !entry.info.local && entry.seen.elapsed() >= stale)
            .map(|entry| entry.info.id.clone())
            .collect();
        for id in &gone {
            members.remove(id);
        }
        gone
    }

    fn lock(&self) -> MutexGuard<'_, HashMap<InstanceId, MemberEntry>> {
        self.members.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}
