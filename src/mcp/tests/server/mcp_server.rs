//! Stopping the hub frees the port at once, so another instance can take it.

use std::net::{Ipv4Addr, TcpListener};
use std::time::Duration;

use crate::support::{FakePort, Harness};

// Binds the port again; panics when it is still taken.
struct PortProbe;

impl PortProbe {
    fn bind(port: u16) -> TcpListener {
        TcpListener::bind((Ipv4Addr::LOCALHOST, port)).expect("the port is free again")
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn stop_frees_the_port_at_once() {
    let harness = Harness::start(FakePort::plain()).await;
    let port = harness.port();
    harness.stop().await;
    drop(PortProbe::bind(port));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn stop_frees_the_port_with_a_session_open() {
    let harness = Harness::start(FakePort::plain()).await;
    let port = harness.port();
    let client = harness.client().await;
    harness.stop().await;
    drop(PortProbe::bind(port));
    drop(client);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn dropping_the_node_frees_the_port() {
    let harness = Harness::start(FakePort::plain()).await;
    let port = harness.port();
    drop(harness);
    let mut waited = Duration::ZERO;
    loop {
        match TcpListener::bind((Ipv4Addr::LOCALHOST, port)) {
            Ok(_) => break,
            Err(error) => {
                assert!(waited < Duration::from_secs(3), "the port is still taken: {error}");
                tokio::time::sleep(Duration::from_millis(20)).await;
                waited += Duration::from_millis(20);
            }
        }
    }
}
