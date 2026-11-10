//! The tools through MCP: the `instance` argument, `"*"`, the control file check and port errors.

use std::sync::Arc;

use headless_input_mcp::PortError;
use serde_json::{Value, json};

use crate::support::{FakePort, FakeSetup, Harness};

const GOOD_FILE: &str = "name = \"square\"\n\nstep \"move\" {\n  hold = [\"forward\"]\n  for  = \"1s\"\n}\n";
const BAD_FILE: &str = "name = \"bad\"\n\nstep \"teleport\" {\n  x = 1\n}\n";

struct Reply;

impl Reply {
    fn json(result: &rmcp::model::CallToolResult) -> Value {
        serde_json::from_str(&Harness::text(result)).expect("reply is JSON")
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn one_instance_needs_no_instance_argument() {
    let harness = Harness::start(FakePort::plain()).await;
    let status = harness.call("status", json!({})).await;
    assert_ne!(status.is_error, Some(true), "{}", Harness::text(&status));
    assert_eq!(Reply::json(&status)["state"], "idle");

    let moved = harness.call("move", json!({ "hold": ["forward"], "for": "2s" })).await;
    assert_ne!(moved.is_error, Some(true), "{}", Harness::text(&moved));
    assert!(harness.fake.calls().iter().any(|call| call.starts_with("live Hold")), "{:?}", harness.fake.calls());
    harness.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn two_instances_without_instance_is_an_error_listing_them() {
    let harness = Harness::start(FakePort::plain()).await;
    let beta = Arc::new(FakePort::plain());
    let member = harness.join("beta", Arc::clone(&beta)).await;

    let result = harness.call("status", json!({})).await;
    assert_eq!(result.is_error, Some(true));
    let text = Harness::text(&result);
    assert!(text.contains("alpha") && text.contains("beta"), "{text}");

    let named = harness.call("status", json!({ "instance": "beta" })).await;
    assert_ne!(named.is_error, Some(true), "{}", Harness::text(&named));
    assert!(beta.calls().contains(&"status".to_owned()));

    let instances = Reply::json(&harness.call("instances", json!({})).await);
    let listed: Vec<(String, bool)> = instances
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| (entry["instance"].as_str().unwrap().to_owned(), entry["hub"].as_bool().unwrap()))
        .collect();
    assert_eq!(listed, [("alpha".to_owned(), true), ("beta".to_owned(), false)]);

    Harness::stop_node(member).await;
    harness.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn star_runs_on_every_instance() {
    let harness = Harness::start(FakePort::plain()).await;
    let beta = Arc::new(FakePort::plain());
    let member = harness.join("beta", Arc::clone(&beta)).await;

    let result = harness.call("run_control_file", json!({ "instance": "*", "text": GOOD_FILE })).await;
    assert_ne!(result.is_error, Some(true), "{}", Harness::text(&result));
    let results = Reply::json(&result);
    let results = results.as_array().unwrap();
    assert_eq!(results.len(), 2, "{results:?}");
    for (entry, id) in results.iter().zip(["alpha", "beta"]) {
        assert_eq!(entry["instance"], id);
        assert_eq!(entry["ok"]["run"], 1, "{entry}");
    }
    assert!(harness.fake.calls().contains(&"run_control_file square".to_owned()));
    assert!(beta.calls().contains(&"run_control_file square".to_owned()));

    let stopped = Reply::json(&harness.call("stop", json!({ "instance": "*" })).await);
    assert_eq!(stopped.as_array().unwrap().len(), 2);
    let refused = harness.call("status", json!({ "instance": "*" })).await;
    assert_eq!(refused.is_error, Some(true));

    Harness::stop_node(member).await;
    harness.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_bad_control_file_is_a_tool_error_with_the_line() {
    let harness = Harness::start(FakePort::plain()).await;
    let result = harness.call("run_control_file", json!({ "text": BAD_FILE })).await;
    assert_eq!(result.is_error, Some(true));
    let text = Harness::text(&result);
    assert!(text.contains("line 3"), "{text}");
    assert!(!harness.fake.calls().iter().any(|call| call.starts_with("run_control_file")));

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("square.hcl");
    std::fs::write(&path, GOOD_FILE).unwrap();
    let from_path = harness.call("run_control_file", json!({ "path": path.display().to_string() })).await;
    assert_ne!(from_path.is_error, Some(true), "{}", Harness::text(&from_path));
    harness.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn busy_port_is_a_tool_error() {
    let fake = FakePort::new(FakeSetup { run_error: Some(PortError::Busy), ..FakeSetup::default() });
    let harness = Harness::start(fake).await;
    let result = harness.call("run_control_file", json!({ "text": GOOD_FILE })).await;
    assert_eq!(result.is_error, Some(true));
    assert_eq!(Harness::text(&result), PortError::Busy.to_string());
    harness.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_bare_name_is_read_from_the_control_folder() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("square.hcl"), GOOD_FILE).unwrap();
    let harness =
        Harness::start(FakePort::new(FakeSetup { folder: dir.path().to_path_buf(), ..FakeSetup::default() })).await;

    let result = harness.call("run_control_file", json!({ "path": "square.hcl" })).await;

    assert_ne!(result.is_error, Some(true), "{}", Harness::text(&result));
    assert!(harness.fake.calls().iter().any(|call| call.starts_with("run_control_file")));
    harness.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_unresolvable_host_is_a_tool_error_naming_it() {
    let harness = Harness::start(FakePort::plain()).await;
    let result = harness.call("join_server", json!({ "address": "name.invalid" })).await;
    assert_eq!(result.is_error, Some(true));
    assert_eq!(Harness::text(&result), "could not resolve name.invalid");
    assert!(!harness.fake.calls().iter().any(|call| call.starts_with("live")), "{:?}", harness.fake.calls());
    harness.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_join_server_by_name_reaches_the_instance_with_the_address() {
    let harness = Harness::start(FakePort::plain()).await;
    let result = harness.call("join_server", json!({ "address": "localhost" })).await;
    assert_ne!(result.is_error, Some(true), "{}", Harness::text(&result));
    assert!(harness.fake.calls().iter().any(|call| call.contains("127.0.0.1")), "{:?}", harness.fake.calls());
    harness.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_control_file_with_an_unresolvable_join_is_refused_before_it_runs() {
    let harness = Harness::start(FakePort::plain()).await;
    let file = "name = \"j\"\n\nstep \"join_server\" {\n  address = \"name.invalid\"\n}\n";
    let result = harness.call("run_control_file", json!({ "text": file })).await;
    assert_eq!(result.is_error, Some(true));
    assert_eq!(Harness::text(&result), "could not resolve name.invalid");
    assert!(!harness.fake.calls().iter().any(|call| call.starts_with("run_control_file")));
    harness.stop().await;
}
