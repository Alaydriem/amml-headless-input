use std::sync::Arc;
use std::time::{Duration, Instant};

use alaydriem_headless_input::services::PairingPrompt;

fn start_asking(prompt: &Arc<PairingPrompt>, wait: Duration) -> std::thread::JoinHandle<bool> {
    let asker = Arc::clone(prompt);
    let asking = std::thread::spawn(move || asker.ask_within("claude-code", wait));
    while !prompt.is_asking() && !asking.is_finished() {
        std::thread::sleep(Duration::from_millis(1));
    }
    asking
}

#[test]
fn pairing_without_an_answer_is_denied() {
    let prompt = Arc::new(PairingPrompt::new());

    let asking = start_asking(&prompt, Duration::from_millis(100));

    assert!(!asking.join().unwrap());
    assert!(!prompt.is_asking());
}

#[test]
fn the_setting_turning_on_allows_the_waiting_question() {
    let prompt = Arc::new(PairingPrompt::new());
    let asking = start_asking(&prompt, Duration::from_secs(5));

    prompt.allow();

    assert!(asking.join().unwrap());
}

#[test]
fn a_switch_with_no_question_waiting_allows_no_later_question() {
    let prompt = Arc::new(PairingPrompt::new());
    prompt.allow();

    let asking = start_asking(&prompt, Duration::from_millis(100));

    assert!(!asking.join().unwrap());
}

#[test]
fn a_second_question_while_one_waits_is_denied() {
    let prompt = Arc::new(PairingPrompt::new());
    let first = start_asking(&prompt, Duration::from_secs(5));

    assert!(!prompt.ask_within("other", Duration::from_secs(5)));

    prompt.allow();
    assert!(first.join().unwrap());
}

#[test]
fn cancel_denies_the_waiting_question_and_later_ones_until_reopen() {
    let prompt = Arc::new(PairingPrompt::new());
    let asking = start_asking(&prompt, Duration::from_secs(5));
    let started = Instant::now();

    prompt.cancel();

    assert!(!asking.join().unwrap());
    assert!(started.elapsed() < Duration::from_secs(1));
    assert!(!prompt.ask_within("claude-code", Duration::from_secs(5)));
    assert!(started.elapsed() < Duration::from_secs(1));
    prompt.reopen();
    let again = start_asking(&prompt, Duration::from_secs(5));
    prompt.allow();
    assert!(again.join().unwrap());
}
