//! The `headless_pair` client: the port from the mod config, the pair request, and the `claude` command line.

use std::net::{Ipv4Addr, TcpListener};

use headless_input_mcp::claude_cli::ClaudeCli;
use headless_input_mcp::pair_client::{PairClient, PairOutcome};

use crate::support::{FakePort, Harness};

#[test]
fn the_port_comes_from_the_config_or_is_the_default() {
    assert_eq!(PairClient::port_from_config("mcp {\n  enabled = true\n  port = 30111\n}\n"), 30111);
    assert_eq!(PairClient::port_from_config("run {\n  watchdog = \"5s\"\n}\n"), 28250);
    assert_eq!(PairClient::port_from_config("not hcl {"), 28250);
    assert_eq!(PairClient::port_from_config("mcp {\n  port = 99999\n}\n"), 28250);
    assert_eq!(PairClient::port_from_config("mcp {\n  port = 80\n}\n"), 28250);
}

#[test]
fn the_config_file_is_in_the_mod_config_folder() {
    let file = PairClient::config_file(std::path::Path::new("M:/mods"));
    assert!(file.ends_with("amml/@alaydriem/headless-input/config/config.hcl"), "{}", file.display());
}

#[test]
fn replies_map_to_outcomes() {
    let ok = b"HTTP/1.1 200 OK\r\ncontent-type: application/json\r\n\r\n{\"token\":\"abc\"}";
    assert_eq!(PairClient::read_reply(ok), PairOutcome::Token("abc".into()));
    assert_eq!(PairClient::read_reply(b"HTTP/1.1 403 Forbidden\r\n\r\n"), PairOutcome::Denied);
    assert_eq!(PairClient::read_reply(b"HTTP/1.1 409 Conflict\r\n\r\n"), PairOutcome::Busy);
    assert!(matches!(PairClient::read_reply(b""), PairOutcome::Failed(_)));
}

#[test]
fn no_server_is_not_running() {
    let port = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap().local_addr().unwrap().port();
    assert_eq!(PairClient::new(port, PairClient::CLIENT).request(), PairOutcome::NotRunning);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn allow_gives_the_client_a_token_and_deny_does_not() {
    let harness = Harness::start(FakePort::pairing(true)).await;
    let port = harness.port();
    let outcome =
        tokio::task::spawn_blocking(move || PairClient::new(port, PairClient::CLIENT).request()).await.unwrap();
    let PairOutcome::Token(token) = outcome else { panic!("no token: {outcome:?}") };
    assert!(harness.tokens().contains(&token));
    assert!(harness.fake.calls().contains(&"approve_pairing claude-code".to_owned()));
    harness.stop().await;

    let harness = Harness::start(FakePort::pairing(false)).await;
    let port = harness.port();
    let outcome = tokio::task::spawn_blocking(move || PairClient::new(port, "other").request()).await.unwrap();
    assert_eq!(outcome, PairOutcome::Denied);
    harness.stop().await;
}

#[test]
fn the_claude_line_names_the_server_url_and_header() {
    assert_eq!(
        ClaudeCli::add_line(28250, "ab12", None),
        "claude mcp add --transport http headless-input http://127.0.0.1:28250/mcp --header \"Authorization: Bearer ab12\""
    );
    assert_eq!(
        ClaudeCli::add_args(28250, "ab12", Some("user")),
        [
            "mcp",
            "add",
            "--scope",
            "user",
            "--transport",
            "http",
            "headless-input",
            "http://127.0.0.1:28250/mcp",
            "--header",
            "Authorization: Bearer ab12"
        ]
    );
    assert_eq!(ClaudeCli::remove_args(Some("user")), ["mcp", "remove", "--scope", "user", "headless-input"]);
}
