use std::sync::Arc;
use std::time::Instant;

use tokio::sync::{Mutex, mpsc};

use super::MemberInfo;
use crate::dto::Forwarded;

// A registered member inside the registry: its info, its bounded command queue and when it was last heard from.
#[derive(Debug)]
pub(crate) struct MemberEntry {
    pub info: MemberInfo,
    pub sender: mpsc::Sender<Forwarded>,
    // one poll at a time takes from the queue
    pub receiver: Arc<Mutex<mpsc::Receiver<Forwarded>>>,
    pub seen: Instant,
    // counts up on every registration, so a closed connection of an earlier registration drops nothing
    pub generation: u64,
    // the polls that wait on the queue now
    pub polls: usize,
}

impl MemberEntry {
    pub fn new(info: MemberInfo, queue: usize, generation: u64) -> Self {
        let (sender, receiver) = mpsc::channel(queue);
        Self { info, sender, receiver: Arc::new(Mutex::new(receiver)), seen: Instant::now(), generation, polls: 0 }
    }
}
