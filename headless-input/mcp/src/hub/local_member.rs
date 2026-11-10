use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use super::{Forwarder, InstanceId, MemberExecutor, MemberRoute};
use crate::port::{HeadlessPort, HostResolver};

// The hub's own instance as a member: takes its commands from the registry queue and runs them on the local port,
// with no HTTP between.
pub struct LocalMember;

impl LocalMember {
    pub async fn run<P: HeadlessPort>(
        port: Arc<P>,
        id: InstanceId,
        forwarder: Arc<Forwarder>,
        resolver: Arc<HostResolver>,
        cancel: CancellationToken,
    ) {
        let registry = Arc::clone(forwarder.registry());
        loop {
            let next = tokio::select! {
                _ = cancel.cancelled() => return,
                next = registry.next(&id, MemberRoute::POLL_WAIT) => next,
            };
            match next {
                Ok(Some(forwarded)) => {
                    let reply =
                        MemberExecutor::execute_blocking(Arc::clone(&port), Arc::clone(&resolver), forwarded.request)
                            .await;
                    forwarder.complete(forwarded.command, reply);
                }
                Ok(None) => {}
                Err(_) => return,
            }
        }
    }
}
