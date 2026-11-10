//! `MemberRegistry`: bounded by `CAPACITY`, and a member whose connection closes leaves.

use std::time::Duration;

use headless_input_core::structs::Capabilities;
use headless_input_mcp::errors::RegistryError;
use headless_input_mcp::hub::{InstanceId, MemberInfo, MemberRegistry};

use crate::support::{Eventually, FakePort, Harness};

struct Member;

impl Member {
    fn remote(id: &str) -> MemberInfo {
        MemberInfo::new(InstanceId::new(id), 1, Capabilities::all(), false)
    }
}

#[test]
fn registry_refuses_past_capacity() {
    let registry = MemberRegistry::new();
    for index in 0..MemberRegistry::CAPACITY {
        registry.register(Member::remote(&format!("member-{index:02}"))).unwrap();
    }
    assert_eq!(registry.len(), MemberRegistry::CAPACITY);

    let refused = registry.register(Member::remote("one-too-many"));
    assert_eq!(refused, Err(RegistryError::Full(MemberRegistry::CAPACITY)));
    assert_eq!(registry.len(), MemberRegistry::CAPACITY);
    assert!(registry.get(&InstanceId::new("one-too-many")).is_none());
    assert!(registry.get(&InstanceId::new("member-00")).is_some(), "nothing is evicted");

    registry.register(Member::remote("member-00")).unwrap();
    assert_eq!(registry.len(), MemberRegistry::CAPACITY, "a member that registers again keeps its place");
}

#[test]
fn a_remote_member_cannot_take_the_id_of_the_hub() {
    let registry = MemberRegistry::new();
    registry.register(MemberInfo::new(InstanceId::new("alpha"), 1, Capabilities::all(), true)).unwrap();
    assert_eq!(registry.register(Member::remote("alpha")), Err(RegistryError::Taken("alpha".to_owned())));
    assert!(!registry.unregister(&InstanceId::new("alpha")));
    assert!(registry.get(&InstanceId::new("alpha")).is_some_and(|info| info.local));
}

#[test]
fn another_process_cannot_take_the_id_of_a_live_member() {
    let registry = MemberRegistry::new();
    let alpha = |pid| MemberInfo::new(InstanceId::new("alpha"), pid, Capabilities::all(), false);
    registry.register(alpha(1)).unwrap();

    assert_eq!(registry.register(alpha(2)), Err(RegistryError::Taken("alpha".to_owned())));
    assert_eq!(registry.get(&InstanceId::new("alpha")).map(|info| info.pid), Some(1));
    assert_eq!(registry.register(alpha(1)), Ok(()), "the same process registers again after a reconnect");
}

#[test]
fn a_member_not_heard_from_is_pruned_but_the_hub_stays() {
    let registry = MemberRegistry::new();
    registry.register(MemberInfo::new(InstanceId::new("alpha"), 1, Capabilities::all(), true)).unwrap();
    registry.register(Member::remote("beta")).unwrap();
    assert_eq!(registry.prune(Duration::from_secs(60)), Vec::<InstanceId>::new());
    assert_eq!(registry.prune(Duration::ZERO), vec![InstanceId::new("beta")]);
    assert_eq!(registry.ids(), ["alpha"]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_closed_connection_drops_the_member() {
    let harness = Harness::start(FakePort::plain()).await;
    let base = format!("http://127.0.0.1:{}", harness.port());
    let client = reqwest::Client::new();
    let registered = client
        .post(format!("{base}/member/register"))
        .bearer_auth(&harness.token)
        .header("content-type", "application/json")
        .body(r#"{"id":"beta","pid":1,"features":{"position":true,"rotation":true,"dimension":true,"join":true,"leave":true}}"#)
        .send()
        .await
        .unwrap();
    assert_eq!(registered.status().as_u16(), 200);
    let hub = harness.node.as_ref().unwrap();
    assert!(hub.members().contains(&"beta".to_owned()));

    let token = harness.token.clone();
    let poll = tokio::spawn(async move {
        reqwest::Client::new().get(format!("{base}/member/poll?id=beta")).bearer_auth(token).send().await
    });
    let open = Eventually::wait(Duration::from_secs(3), || hub.polling().contains(&"beta".to_owned())).await;
    assert!(open, "the poll never reached the hub: {:?}", hub.polling());
    poll.abort();

    let gone = Eventually::wait(MemberRegistry::STALE / 2, || !hub.members().contains(&"beta".to_owned())).await;
    assert!(gone, "the member is still listed: {:?}", hub.members());
    harness.stop().await;
}
