//! `HubNode`: real nodes in one process on a free port. Exactly one binds, the others register, one takes over
//! when the hub stops, a foreign program on the port turns MCP off, and a silent hub is marked stalled.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use axum::Json;
use axum::http::StatusCode;
use headless_input_mcp::dto::HubHello;
use headless_input_mcp::hub::HubSupervisor;
use headless_input_mcp::{HubNode, HubRole};
use tempfile::TempDir;

use crate::support::{Eventually, FakePort, FreePort, Harness, ResetServer, SideServer};

const SETTLE: Duration = Duration::from_secs(10);

// Nodes on one port over one token folder.
struct Cluster {
    port: u16,
    dir: TempDir,
}

impl Cluster {
    fn new() -> Self {
        Self { port: FreePort::pick(), dir: tempfile::tempdir().unwrap() }
    }

    fn node(&self, id: &str) -> HubNode {
        Harness::node(self.port, id, Arc::new(FakePort::plain()), self.dir.path())
    }

    fn roles(nodes: &[&HubNode]) -> (usize, usize) {
        let hubs = nodes.iter().filter(|node| node.role() == HubRole::Hub).count();
        let members = nodes.iter().filter(|node| node.role() == HubRole::Member).count();
        (hubs, members)
    }

    fn hub<'a>(nodes: &[&'a HubNode]) -> &'a HubNode {
        nodes.iter().find(|node| node.role() == HubRole::Hub).copied().expect("a hub")
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn exactly_one_of_three_nodes_becomes_hub() {
    let cluster = Arc::new(Cluster::new());
    let handle = tokio::runtime::Handle::current();
    let barrier = Arc::new(std::sync::Barrier::new(3));
    let starts: Vec<_> = ["a", "b", "c"]
        .into_iter()
        .map(|id| {
            let (cluster, barrier, handle) = (Arc::clone(&cluster), Arc::clone(&barrier), handle.clone());
            std::thread::spawn(move || {
                let _entered = handle.enter();
                barrier.wait();
                cluster.node(id)
            })
        })
        .collect();
    let nodes: Vec<HubNode> = starts.into_iter().map(|start| start.join().unwrap()).collect();
    let refs: Vec<&HubNode> = nodes.iter().collect();

    assert!(Eventually::wait(SETTLE, || Cluster::roles(&refs) == (1, 2)).await, "roles {:?}", Cluster::roles(&refs));
    assert_eq!(Cluster::hub(&refs).members(), ["a", "b", "c"]);
    for node in nodes {
        Harness::stop_node(node).await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn members_register_with_the_hub() {
    let cluster = Cluster::new();
    let hub = cluster.node("a");
    assert_eq!(hub.role(), HubRole::Hub);
    let b = cluster.node("b");
    let c = cluster.node("c");
    assert!(Eventually::wait(SETTLE, || hub.members() == ["a", "b", "c"]).await, "members {:?}", hub.members());
    assert_eq!((b.role(), c.role()), (HubRole::Member, HubRole::Member));
    assert!(b.members().is_empty(), "a member serves no one");

    Harness::stop_node(c).await;
    assert!(Eventually::wait(Duration::from_secs(2), || hub.members() == ["a", "b"]).await, "{:?}", hub.members());
    Harness::stop_node(b).await;
    Harness::stop_node(hub).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_member_with_the_id_of_the_hub_registers_under_the_id_and_its_pid() {
    let cluster = Cluster::new();
    let hub = cluster.node("a");
    assert_eq!(hub.role(), HubRole::Hub);
    let twin = cluster.node("a");
    let renamed = format!("a-{}", std::process::id());

    let joined = Eventually::wait(SETTLE, || hub.members() == ["a".to_string(), renamed.clone()]).await;

    assert!(joined, "members {:?}", hub.members());
    assert_eq!(twin.role(), HubRole::Member);
    Harness::stop_node(twin).await;
    Harness::stop_node(hub).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn when_the_hub_stops_one_member_takes_over_and_the_rest_reregister() {
    let cluster = Cluster::new();
    let hub = cluster.node("a");
    let b = cluster.node("b");
    let c = cluster.node("c");
    assert!(Eventually::wait(SETTLE, || hub.members() == ["a", "b", "c"]).await, "members {:?}", hub.members());

    Harness::stop_node(hub).await;
    let rest = [&b, &c];
    assert!(Eventually::wait(SETTLE, || Cluster::roles(&rest) == (1, 1)).await, "roles {:?}", Cluster::roles(&rest));
    let new_hub = Cluster::hub(&rest);
    assert!(Eventually::wait(SETTLE, || new_hub.members() == ["b", "c"]).await, "members {:?}", new_hub.members());
    Harness::stop_node(b).await;
    Harness::stop_node(c).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_foreign_server_on_the_port_turns_mcp_off() {
    let cluster = Cluster::new();
    let hits = Arc::new(AtomicUsize::new(0));
    let counted = Arc::clone(&hits);
    let foreign = axum::Router::new().fallback(move || {
        let counted = Arc::clone(&counted);
        async move {
            counted.fetch_add(1, Ordering::SeqCst);
            (StatusCode::OK, r#"{"server":"another-program","protocol":1}"#)
        }
    });
    let server = SideServer::start(cluster.port, foreign);

    let node = cluster.node("a");
    assert!(Eventually::wait(SETTLE, || hits.load(Ordering::SeqCst) >= 1).await, "the node never asked");
    tokio::time::sleep(Duration::from_millis(1500)).await;
    let asked = hits.load(Ordering::SeqCst);
    assert_eq!(asked, 1, "the node asks once and stays off");
    assert_eq!(node.role(), HubRole::Off);
    assert!(node.members().is_empty());

    Harness::stop_node(node).await;
    server.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_silent_hub_is_marked_stalled_after_five_seconds() {
    let cluster = Cluster::new();
    let silent = axum::Router::new()
        .route("/member/hello", axum::routing::get(|| async { Json(HubHello::new()) }))
        .route("/member/register", axum::routing::post(|| async { Json(HubHello::new()) }))
        .fallback(std::future::pending::<StatusCode>);
    let server = SideServer::start(cluster.port, silent);

    let node = cluster.node("a");
    assert!(Eventually::wait(SETTLE, || node.role() == HubRole::Member).await, "role {:?}", node.role());
    let registered = Instant::now();
    assert!(Eventually::wait(SETTLE, || node.role() == HubRole::Stalled).await, "role {:?}", node.role());
    let after = registered.elapsed();
    assert!(after >= Duration::from_millis(4500), "stalled after {after:?}");
    assert!(after <= Duration::from_secs(8), "stalled after {after:?}");

    server.stop();
    assert!(
        Eventually::wait(SETTLE, || node.role() == HubRole::Hub).await,
        "the stalled hub let the port go, but the node is {:?}",
        node.role()
    );
    Harness::stop_node(node).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_hub_that_resets_the_connection_does_not_turn_mcp_off() {
    let cluster = Cluster::new();
    let server = ResetServer::start(cluster.port, 3);
    let node = cluster.node("a");
    assert!(Eventually::wait(SETTLE, || server.accepted() == 3).await, "accepted {}", server.accepted());
    server.stop();
    assert!(
        Eventually::wait(SETTLE, || node.role() == HubRole::Hub).await,
        "the port came free, but the node is {:?}",
        node.role()
    );
    Harness::stop_node(node).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_program_that_always_breaks_the_connection_turns_mcp_off_after_the_limit() {
    let cluster = Cluster::new();
    let limit = HubSupervisor::<FakePort>::GARBLED_LIMIT as usize;
    let server = ResetServer::start(cluster.port, usize::MAX);
    let node = cluster.node("a");
    assert!(Eventually::wait(SETTLE, || server.accepted() >= limit).await, "accepted {}", server.accepted());
    tokio::time::sleep(Duration::from_millis(1000)).await;
    assert_eq!(server.accepted(), limit, "the node stops asking at the limit");
    assert_eq!(node.role(), HubRole::Off);
    Harness::stop_node(node).await;
    server.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_401_on_heartbeat_leads_to_registration_again() {
    let cluster = Cluster::new();
    let registrations = Arc::new(AtomicUsize::new(0));
    let counted = Arc::clone(&registrations);
    let hub = axum::Router::new()
        .route("/member/hello", axum::routing::get(|| async { Json(HubHello::new()) }))
        .route(
            "/member/register",
            axum::routing::post(move || {
                let counted = Arc::clone(&counted);
                async move {
                    counted.fetch_add(1, Ordering::SeqCst);
                    Json(HubHello::new())
                }
            }),
        )
        .route("/member/heartbeat", axum::routing::post(|| async { StatusCode::UNAUTHORIZED }))
        .fallback(std::future::pending::<StatusCode>);
    let server = SideServer::start(cluster.port, hub);
    let node = cluster.node("a");
    assert!(
        Eventually::wait(SETTLE, || registrations.load(Ordering::SeqCst) >= 2).await,
        "registrations {}",
        registrations.load(Ordering::SeqCst)
    );
    assert_ne!(node.role(), HubRole::Stalled);
    Harness::stop_node(node).await;
    server.stop();
}
