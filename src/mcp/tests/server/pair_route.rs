//! `POST /pair`: the route with no bearer check. Allow issues a saved token, deny is 403, a second request while
//! the first waits for its answer is 409, and a bad body is 400.

use std::time::Duration;

use headless_input_mcp::token::TokenStore;

use crate::support::{Eventually, FakePort, FakeSetup, Harness};

const INIT_BODY: &str = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}"#;

// Raw requests to the two routes.
struct Http;

impl Http {
    async fn pair(url: &str, body: &str) -> (u16, String) {
        let response = reqwest::Client::new()
            .post(url)
            .header("content-type", "application/json")
            .body(body.to_owned())
            .send()
            .await
            .expect("pair request");
        let status = response.status().as_u16();
        (status, response.text().await.expect("body"))
    }

    async fn mcp(harness: &Harness, token: Option<&str>) -> u16 {
        let mut request = reqwest::Client::new()
            .post(harness.url())
            .header("content-type", "application/json")
            .header("accept", "application/json, text/event-stream")
            .body(INIT_BODY);
        if let Some(token) = token {
            request = request.header("authorization", format!("Bearer {token}"));
        }
        request.send().await.expect("mcp request").status().as_u16()
    }

    fn token_of(body: &str) -> String {
        let value: serde_json::Value = serde_json::from_str(body).expect("reply is JSON");
        value["token"].as_str().expect("a token").to_owned()
    }

    fn saved_count(harness: &Harness) -> usize {
        let text = std::fs::read_to_string(harness.dir.path().join(TokenStore::FILE_NAME)).expect("token file");
        let value: serde_json::Value = serde_json::from_str(&text).expect("token file is JSON");
        value["tokens"].as_array().expect("tokens list").len()
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn pair_allow_issues_a_saved_token() {
    let harness = Harness::start(FakePort::pairing(true)).await;
    let (status, body) = Http::pair(&harness.pair_url(), r#"{"client":"claude-code"}"#).await;
    assert_eq!(status, 200, "{body}");
    let token = Http::token_of(&body);
    assert_eq!(token.len(), 64, "bad token {token}");
    assert_ne!(token, harness.token);
    assert!(harness.fake.calls().contains(&"approve_pairing claude-code".to_owned()));

    assert!(TokenStore::load(harness.dir.path()).contains(&token));
    let status = Http::mcp(&harness, Some(&token)).await;
    assert!((200..300).contains(&status), "status was {status}");
    harness.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn pair_deny_is_403_and_saves_nothing() {
    let harness = Harness::start(FakePort::pairing(false)).await;
    let before = Http::saved_count(&harness);
    let (status, body) = Http::pair(&harness.pair_url(), r#"{"client":"claude-code"}"#).await;
    assert_eq!(status, 403, "{body}");
    assert!(!body.contains("token"), "{body}");
    assert_eq!(Http::saved_count(&harness), before);
    harness.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_second_pair_while_the_first_waits_is_409() {
    let (open, gate) = std::sync::mpsc::channel();
    let fake = FakePort::new(FakeSetup { pairing: true, pairing_gate: Some(gate), ..FakeSetup::default() });
    let harness = Harness::start(fake).await;
    let url = harness.pair_url();
    let first = tokio::spawn(async move { Http::pair(&url, r#"{"client":"first"}"#).await });
    assert!(
        Eventually::wait(Duration::from_secs(5), || harness.fake.calls().contains(&"approve_pairing first".to_owned()))
            .await
    );
    let (status, _) = Http::pair(&harness.pair_url(), r#"{"client":"second"}"#).await;
    assert_eq!(status, 409);
    open.send(()).unwrap();
    assert_eq!(first.await.unwrap().0, 200);
    assert!(!harness.fake.calls().contains(&"approve_pairing second".to_owned()));
    harness.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn pair_bad_body_is_400() {
    let harness = Harness::start(FakePort::pairing(true)).await;
    let long = format!(r#"{{"client":"{}"}}"#, "a".repeat(65));
    for body in ["", "not json", r#"{"client":""}"#, r#"{"name":"x"}"#, long.as_str(), r#"{"client":"a\nb"}"#] {
        let (status, reply) = Http::pair(&harness.pair_url(), body).await;
        assert_eq!(status, 400, "body {body:?} gave {reply}");
    }
    let at_most = format!(r#"{{"client":"{}"}}"#, "a".repeat(64));
    assert_eq!(Http::pair(&harness.pair_url(), &at_most).await.0, 200);
    assert_eq!(harness.fake.calls().iter().filter(|call| call.starts_with("approve_pairing")).count(), 1);
    harness.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn revoked_token_is_401() {
    let harness = Harness::start(FakePort::plain()).await;
    let token = harness.token.clone();
    let status = Http::mcp(&harness, Some(&token)).await;
    assert!((200..300).contains(&status), "status was {status}");
    harness.tokens().revoke_all().expect("revoked");
    assert_eq!(Http::mcp(&harness, Some(&token)).await, 401);
    harness.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn pair_route_needs_no_token_but_mcp_does() {
    let harness = Harness::start(FakePort::pairing(false)).await;
    assert_eq!(Http::pair(&harness.pair_url(), r#"{"client":"claude-code"}"#).await.0, 403);
    assert_eq!(Http::mcp(&harness, None).await, 401);
    harness.stop().await;
}
