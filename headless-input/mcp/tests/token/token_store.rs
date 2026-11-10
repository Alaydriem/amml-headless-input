//! `TokenStore`: the tokens issued by pairing, kept in the shared `mcp.tokens.json`.

use headless_input_mcp::token::TokenStore;

// The shape every token must have: 64 lowercase hex characters.
struct TokenShape;

impl TokenShape {
    fn is_valid(token: &str) -> bool {
        token.len() == 64 && token.chars().all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c))
    }
}

#[test]
fn a_new_folder_has_no_tokens_and_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let store = TokenStore::load(dir.path());
    assert!(!store.contains(&"a".repeat(64)));
    assert!(!store.contains(""));
    assert!(!dir.path().join(TokenStore::FILE_NAME).exists());
}

#[test]
fn an_issued_token_is_saved_and_kept_across_loads() {
    let dir = tempfile::tempdir().unwrap();
    let store = TokenStore::load(dir.path());
    let token = store.issue("claude-code").unwrap();
    assert!(TokenShape::is_valid(&token), "bad token {token}");
    assert!(store.contains(&token));

    let text = std::fs::read_to_string(dir.path().join("mcp.tokens.json")).unwrap();
    let value: serde_json::Value = serde_json::from_str(&text).unwrap();
    let saved = &value["tokens"][0];
    assert_eq!(saved["token"], token);
    assert_eq!(saved["client"], "claude-code");
    assert!(saved["issued_ms"].as_u64().unwrap() > 0);

    assert!(TokenStore::load(dir.path()).contains(&token));
}

#[test]
fn two_tokens_differ_and_both_open() {
    let dir = tempfile::tempdir().unwrap();
    let store = TokenStore::load(dir.path());
    let first = store.issue("a").unwrap();
    let second = store.issue("b").unwrap();
    assert_ne!(first, second);
    assert!(store.contains(&first) && store.contains(&second));
    assert!(!store.contains(&first[..63]), "a prefix is not a token");
}

#[test]
fn revoke_all_clears_memory_and_file() {
    let dir = tempfile::tempdir().unwrap();
    let store = TokenStore::load(dir.path());
    let token = store.issue("claude-code").unwrap();
    store.revoke_all().unwrap();
    assert!(!store.contains(&token));
    assert!(!TokenStore::load(dir.path()).contains(&token));
}

#[test]
fn a_broken_file_loads_as_no_tokens() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join(TokenStore::FILE_NAME), "not json").unwrap();
    let store = TokenStore::load(dir.path());
    assert!(!store.contains("not json"));
    let token = store.issue("claude-code").unwrap();
    assert!(TokenStore::load(dir.path()).contains(&token));
}

#[test]
fn a_token_issued_by_another_instance_opens_this_one_and_its_revoke_closes_it() {
    let dir = tempfile::tempdir().unwrap();
    let here = TokenStore::load(dir.path());
    let there = TokenStore::load(dir.path());
    let token = there.issue("claude-code").unwrap();
    assert!(here.contains(&token));
    there.revoke_all().unwrap();
    assert!(!here.contains(&token));
}

#[test]
fn a_revoke_in_another_instance_closes_this_one_when_the_file_time_did_not_change() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join(TokenStore::FILE_NAME);
    let here = TokenStore::load(dir.path());
    let there = TokenStore::load(dir.path());
    let token = there.issue("claude-code").unwrap();
    assert!(here.contains(&token));
    let issued_at = std::fs::metadata(&file).unwrap().modified().unwrap();
    there.revoke_all().unwrap();
    std::fs::File::options().write(true).open(&file).unwrap().set_modified(issued_at).unwrap();
    assert!(!here.contains(&token));
}

#[test]
fn issuing_keeps_the_tokens_another_instance_saved() {
    let dir = tempfile::tempdir().unwrap();
    let here = TokenStore::load(dir.path());
    let there = TokenStore::load(dir.path());
    let theirs = there.issue("a").unwrap();
    let ours = here.issue("b").unwrap();
    let fresh = TokenStore::load(dir.path());
    assert!(fresh.contains(&theirs) && fresh.contains(&ours));
}

#[test]
fn the_store_drops_the_oldest_token_past_capacity() {
    let dir = tempfile::tempdir().unwrap();
    let store = TokenStore::load(dir.path());
    let first = store.issue("first").unwrap();
    let second = store.issue("second").unwrap();
    for index in 2..TokenStore::CAPACITY {
        store.issue(&format!("client-{index}")).unwrap();
    }
    assert_eq!(store.len(), TokenStore::CAPACITY);
    assert!(store.contains(&first));

    let newest = store.issue("newest").unwrap();
    assert_eq!(store.len(), TokenStore::CAPACITY);
    assert!(!store.contains(&first), "the oldest token is dropped");
    assert!(store.contains(&second) && store.contains(&newest));
    assert_eq!(TokenStore::load(dir.path()).len(), TokenStore::CAPACITY);
}

#[test]
fn the_member_token_is_issued_once_and_shared() {
    let dir = tempfile::tempdir().unwrap();
    let here = TokenStore::load(dir.path());
    let there = TokenStore::load(dir.path());
    let token = here.member_token().unwrap();
    assert_eq!(there.member_token().unwrap(), token);
    assert_eq!(TokenStore::load(dir.path()).len(), 1);
}

#[test]
fn member_tokens_never_evict_a_paired_token() {
    let dir = tempfile::tempdir().unwrap();
    let store = TokenStore::load(dir.path());
    let paired: Vec<String> =
        (0..TokenStore::CAPACITY).map(|index| store.issue(&format!("client-{index}")).unwrap()).collect();
    let file = dir.path().join(TokenStore::FILE_NAME);
    for _ in 0..40 {
        TokenStore::load(dir.path()).member_token().unwrap();
        let mut saved: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
        saved["tokens"].as_array_mut().unwrap().retain(|token| token["member"] != true);
        std::fs::write(&file, saved.to_string()).unwrap();
    }
    let member = store.member_token().unwrap();
    let fresh = TokenStore::load(dir.path());
    assert!(paired.iter().all(|token| fresh.contains(token)), "a paired token was evicted");
    assert!(fresh.contains(&member));
    assert_eq!(fresh.len(), TokenStore::CAPACITY + 1);

    let newest = store.issue("newest").unwrap();
    let fresh = TokenStore::load(dir.path());
    assert!(fresh.contains(&member), "a pairing token never evicts the member token");
    assert!(!fresh.contains(&paired[0]), "the oldest pairing token goes");
    assert!(paired[1..].iter().all(|token| fresh.contains(token)) && fresh.contains(&newest));
    assert_eq!(fresh.len(), TokenStore::CAPACITY + 1);
}
