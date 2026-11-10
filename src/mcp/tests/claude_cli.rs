//! `ClaudeCli`: a missing `claude` gives the line to run, and a `claude` that hangs is stopped after the time-out.
//! The fake programs are small `.cmd` files, so no real `claude` runs.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use headless_input_mcp::claude_cli::{ClaudeCli, ClaudeOutcome};

// A fake `claude` in a temp folder.
struct FakeClaude;

impl FakeClaude {
    #[cfg_attr(not(windows), allow(dead_code))]
    fn write(dir: &Path, name: &str, body: &str) -> String {
        let file: PathBuf = dir.join(name);
        std::fs::write(&file, format!("@echo off\r\n{body}\r\n")).unwrap();
        file.display().to_string()
    }
}

#[test]
fn the_user_scope_line_names_the_server() {
    let line = ClaudeCli::add_line(28250, "ab", Some(ClaudeCli::SCOPE));
    assert!(line.starts_with("claude mcp add --scope user --transport http headless-input"), "{line}");
}

#[test]
fn a_missing_claude_gives_the_line_to_run() {
    let cli = ClaudeCli::new(vec!["headless-no-such-claude-program".into()], ClaudeCli::TIMEOUT);
    assert_eq!(
        cli.register(28250, "ab", Some(ClaudeCli::SCOPE)),
        ClaudeOutcome::NotFound(ClaudeCli::add_line(28250, "ab", Some(ClaudeCli::SCOPE)))
    );
}

#[cfg(windows)]
#[test]
fn a_claude_that_ends_well_adds_and_one_that_fails_gives_its_words() {
    let dir = tempfile::tempdir().unwrap();
    let good = FakeClaude::write(dir.path(), "good.cmd", "exit /b 0");
    assert_eq!(
        ClaudeCli::new(vec![good], ClaudeCli::TIMEOUT).register(28250, "ab", Some("user")),
        ClaudeOutcome::Added
    );
    let bad = FakeClaude::write(
        dir.path(),
        "bad.cmd",
        "if \"%1\"==\"--version\" exit /b 0\r\necho no access 1>&2\r\nexit /b 1",
    );
    assert_eq!(
        ClaudeCli::new(vec![bad], ClaudeCli::TIMEOUT).register(28250, "ab", Some("user")),
        ClaudeOutcome::Failed("no access".into())
    );
}

#[cfg(windows)]
#[test]
fn a_claude_that_hangs_is_stopped_after_the_time_out() {
    let dir = tempfile::tempdir().unwrap();
    let slow = FakeClaude::write(dir.path(), "slow.cmd", "ping -n 30 127.0.0.1 >nul");
    let cli = ClaudeCli::new(vec![slow.clone()], Duration::from_millis(300));

    let start = Instant::now();
    let error = cli.run(&slow, &["mcp".into()]).expect_err("a time-out");
    assert_eq!(error.kind(), std::io::ErrorKind::TimedOut);
    assert!(start.elapsed() < Duration::from_secs(5), "took {:?}", start.elapsed());

    let start = Instant::now();
    match cli.register(28250, "ab", Some("user")) {
        ClaudeOutcome::Failed(reason) => assert!(reason.contains("did not finish"), "{reason}"),
        other => panic!("not a failure: {other:?}"),
    }
    assert!(start.elapsed() < Duration::from_secs(10), "took {:?}", start.elapsed());
}
