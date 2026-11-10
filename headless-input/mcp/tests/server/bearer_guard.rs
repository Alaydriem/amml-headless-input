//! The bearer check on `/mcp` and the member routes.

use crate::support::{FakePort, Harness};

const INIT_BODY: &str = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}"#;

// Sends one raw `initialize` POST with an optional `Authorization` header.
struct RawPost;

impl RawPost {
    async fn send(url: &str, authorization: Option<&str>) -> u16 {
        let mut request = reqwest::Client::new()
            .post(url)
            .header("content-type", "application/json")
            .header("accept", "application/json, text/event-stream")
            .body(INIT_BODY);
        if let Some(value) = authorization {
            request = request.header("authorization", value);
        }
        request.send().await.expect("http request").status().as_u16()
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn request_without_token_is_401() {
    let harness = Harness::start(FakePort::plain()).await;
    assert_eq!(RawPost::send(&harness.url(), None).await, 401);
    harness.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn request_with_wrong_token_is_401() {
    let harness = Harness::start(FakePort::plain()).await;
    let wrong = format!("Bearer {}", "f".repeat(64));
    assert_eq!(RawPost::send(&harness.url(), Some(&wrong)).await, 401);
    let not_bearer = format!("Basic {}", harness.token);
    assert_eq!(RawPost::send(&harness.url(), Some(&not_bearer)).await, 401);
    harness.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn request_with_token_is_accepted() {
    let harness = Harness::start(FakePort::plain()).await;
    let right = format!("Bearer {}", harness.token);
    let status = RawPost::send(&harness.url(), Some(&right)).await;
    assert!((200..300).contains(&status), "status was {status}");
    harness.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn member_routes_need_the_token_but_hello_does_not() {
    let harness = Harness::start(FakePort::plain()).await;
    let base = format!("http://127.0.0.1:{}", harness.port());
    let client = reqwest::Client::new();
    let register = client
        .post(format!("{base}/member/register"))
        .header("content-type", "application/json")
        .body(r#"{"id":"beta","pid":1,"features":{"position":true,"rotation":true,"dimension":true,"join":true,"leave":true}}"#)
        .send()
        .await
        .unwrap();
    assert_eq!(register.status().as_u16(), 401);
    let hello = client.get(format!("{base}/member/hello")).send().await.unwrap();
    assert_eq!(hello.status().as_u16(), 200);
    let body: serde_json::Value = serde_json::from_str(&hello.text().await.unwrap()).unwrap();
    assert_eq!(body["server"], "headless-input");
    assert_eq!(body["protocol"], 1);
    harness.stop().await;
}
