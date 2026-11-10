use std::net::TcpListener;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use alaydriem_headless_input::services::{HeadlessRuntime, HeadlessTokio, HubControl, NodeLauncher};
use alaydriem_headless_input::traits::HubLauncher;
use headless_input_mcp::{HubRole, InstanceId};

use crate::support::RunHarness;
use crate::support::fake_launcher::FakeLauncher;

const SETTLE: Duration = Duration::from_secs(10);

fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port()
}

// follows the settings once a frame until `done` holds
fn follow_until<L: HubLauncher>(
    hub: &mut HubControl<L>,
    enabled: bool,
    port: u16,
    done: impl Fn(&HubControl<L>) -> bool,
) -> bool {
    let deadline = Instant::now() + SETTLE;
    while Instant::now() < deadline {
        hub.follow(enabled, port, Instant::now());
        if done(hub) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    false
}

fn hub(tokio: &HeadlessTokio, harness: &RunHarness, tokens: &tempfile::TempDir) -> HubControl {
    let launcher = NodeLauncher::new(tokio.handle(), harness.port(), InstanceId::new("alpha"), tokens.path());
    HubControl::new(tokio.handle(), launcher)
}

#[test]
fn the_hub_follows_mcp_enabled() {
    let tokio = HeadlessTokio::start().unwrap();
    let harness = RunHarness::new();
    let tokens = tempfile::tempdir().unwrap();
    let mut hub = hub(&tokio, &harness, &tokens);
    let port = free_port();

    assert!(follow_until(&mut hub, true, port, |hub| hub.role() == Some(HubRole::Hub)), "{:?}", hub.role());
    assert!(TcpListener::bind(("127.0.0.1", port)).is_err());

    assert!(follow_until(&mut hub, false, port, |hub| hub.role().is_none() && !hub.is_switching()));
    assert!(TcpListener::bind(("127.0.0.1", port)).is_ok());

    hub.stop();
    tokio.stop();
}

#[test]
fn stop_frees_the_hub_port() {
    let tokio = HeadlessTokio::start().unwrap();
    let harness = RunHarness::new();
    let tokens = tempfile::tempdir().unwrap();
    let mut hub = hub(&tokio, &harness, &tokens);
    let port = free_port();
    assert!(follow_until(&mut hub, true, port, |hub| hub.role() == Some(HubRole::Hub)));

    hub.stop();
    tokio.stop();

    assert!(TcpListener::bind(("127.0.0.1", port)).is_ok());
}

#[test]
fn an_in_flight_switch_during_stop_leaves_no_live_node() {
    let tokio = HeadlessTokio::start().unwrap();
    let launcher = FakeLauncher::new(HubControl::<FakeLauncher>::SWITCH_WAIT * 2);
    let live = launcher.live.clone();
    let starts = launcher.starts.clone();
    let mut hub = HubControl::new(tokio.handle(), launcher);
    hub.follow(true, 28250, Instant::now());
    while starts.load(Ordering::SeqCst) == 0 {
        std::thread::sleep(Duration::from_millis(1));
    }

    hub.stop();

    let deadline = Instant::now() + SETTLE;
    while hub.is_switching() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(!hub.is_switching());
    assert_eq!(live.load(Ordering::SeqCst), 0);
    assert_eq!(hub.role(), None);
    tokio.stop();
}

#[test]
fn stop_returns_within_the_bound_with_a_node_that_ignores_stop() {
    let tokio = HeadlessTokio::start().unwrap();
    let launcher = FakeLauncher::new(Duration::ZERO);
    let live = launcher.live.clone();
    let mut hub = HubControl::new(tokio.handle(), launcher);
    assert!(follow_until(&mut hub, true, 28250, |hub| hub.role() == Some(HubRole::Hub)));

    let started = Instant::now();
    hub.stop();
    tokio.stop();

    assert!(started.elapsed() <= HeadlessRuntime::DISABLE_BOUND, "{:?}", started.elapsed());
    assert!(HeadlessRuntime::DISABLE_BOUND <= Duration::from_secs(3));
    assert_eq!(live.load(Ordering::SeqCst), 0);
}
