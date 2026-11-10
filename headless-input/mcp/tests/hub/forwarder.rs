//! `Forwarder`: a command reaches the member it names, the reply comes back, and the queues and the pending map
//! are bounded.

use std::sync::Arc;
use std::time::Duration;

use headless_input_core::structs::Capabilities;
use headless_input_mcp::PortError;
use headless_input_mcp::dto::{MemberReply, MemberRequest};
use headless_input_mcp::hub::{Forwarder, InstanceId, MemberInfo, MemberRegistry};

use crate::support::Eventually;

struct Members;

impl Members {
    fn registry(ids: &[&str]) -> Arc<MemberRegistry> {
        let registry = MemberRegistry::new_shared();
        for id in ids {
            registry.register(MemberInfo::new(InstanceId::new(*id), 1, Capabilities::all(), false)).unwrap();
        }
        registry
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_command_reaches_the_named_member_and_returns_its_reply() {
    let registry = Members::registry(&["a", "b"]);
    let forwarder = Forwarder::new_shared(Arc::clone(&registry), Forwarder::TIMEOUT);
    let answering = Arc::clone(&forwarder);
    let queue = Arc::clone(&registry);
    let member = tokio::spawn(async move {
        let forwarded = queue.next(&InstanceId::new("b"), Duration::from_secs(5)).await.unwrap().unwrap();
        assert_eq!(forwarded.request, MemberRequest::Stop);
        answering.complete(forwarded.command, MemberReply::Done)
    });

    let reply = forwarder.send(&InstanceId::new("b"), MemberRequest::Stop).await;
    assert_eq!(reply, Ok(MemberReply::Done));
    assert!(member.await.unwrap());
    assert_eq!(registry.next(&InstanceId::new("a"), Duration::from_millis(20)).await, Ok(None));
    assert_eq!(forwarder.pending(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_unanswered_command_times_out() {
    let registry = Members::registry(&["a"]);
    let forwarder = Forwarder::new(Arc::clone(&registry), Duration::from_millis(200));
    let reply = forwarder.send(&InstanceId::new("a"), MemberRequest::Status).await;
    assert!(matches!(reply, Err(PortError::Timeout(_))), "{reply:?}");
    assert_eq!(forwarder.pending(), 0);
    let late = registry.next(&InstanceId::new("a"), Duration::from_millis(20)).await.unwrap().unwrap();
    assert!(!forwarder.complete(late.command, MemberReply::Done), "a late reply finds no command");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_unknown_member_is_an_error() {
    let forwarder = Forwarder::new(Members::registry(&["a"]), Forwarder::TIMEOUT);
    let reply = forwarder.send(&InstanceId::new("zed"), MemberRequest::Status).await;
    assert_eq!(reply, Err(PortError::UnknownInstance("zed".to_owned())));
    assert_eq!(forwarder.pending(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_full_member_queue_is_busy() {
    let registry = Members::registry(&["a"]);
    let forwarder = Forwarder::new_shared(registry, Duration::from_secs(5));
    for _ in 0..MemberRegistry::QUEUE {
        let waiting = Arc::clone(&forwarder);
        tokio::spawn(async move { waiting.send(&InstanceId::new("a"), MemberRequest::Status).await });
    }
    assert!(Eventually::wait(Duration::from_secs(3), || forwarder.pending() == MemberRegistry::QUEUE).await);
    let reply = forwarder.send(&InstanceId::new("a"), MemberRequest::Status).await;
    assert_eq!(reply, Err(PortError::Busy));
    assert_eq!(forwarder.pending(), MemberRegistry::QUEUE);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_pending_map_refuses_past_capacity() {
    let full: Vec<String> =
        (0..Forwarder::PENDING_CAPACITY / MemberRegistry::QUEUE).map(|index| format!("m{index:02}")).collect();
    let mut ids: Vec<&str> = full.iter().map(String::as_str).collect();
    ids.push("spare");
    let registry = Members::registry(&ids);
    let forwarder = Forwarder::new_shared(Arc::clone(&registry), Duration::from_secs(5));
    for id in &full {
        for _ in 0..MemberRegistry::QUEUE {
            let (waiting, id) = (Arc::clone(&forwarder), InstanceId::new(id.as_str()));
            tokio::spawn(async move { waiting.send(&id, MemberRequest::Status).await });
        }
    }
    assert!(Eventually::wait(Duration::from_secs(3), || forwarder.pending() == Forwarder::PENDING_CAPACITY).await);

    let reply = forwarder.send(&InstanceId::new("spare"), MemberRequest::Status).await;
    assert_eq!(reply, Err(PortError::Busy));
    assert_eq!(forwarder.pending(), Forwarder::PENDING_CAPACITY);
    let queued = registry.next(&InstanceId::new("spare"), Duration::from_millis(20)).await;
    assert_eq!(queued, Ok(None), "the refused command never reached the member queue");
}
