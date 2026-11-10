use alaydriem_headless_input::HeadlessMod;
use alaydriem_headless_input::game::HeadlessEntry;
use alaydriem_headless_input::services::{HeadlessRuntime, HookInstaller};
use amml_game::EntryNeed;
use headless_input_core::structs::Capabilities;

use crate::support::fake_host::FakeHost;

fn runtime() -> HeadlessRuntime {
    HeadlessRuntime::new(&crate::support::RuntimeDirs::dirs())
}

fn required_entries() -> Vec<(&'static str, u64)> {
    let mut entries = vec![(HeadlessMod::UPDATE, 0x1000), (HeadlessMod::TICK, 0x2000)];
    entries.extend(
        HeadlessMod::wanted()
            .iter()
            .filter(|(_, need)| matches!(need, EntryNeed::Required))
            .map(|(id, _)| (id.as_str(), 0x10)),
    );
    entries
}

#[test]
fn the_client_update_and_tick_hooks_are_installed_in_order() {
    let host = FakeHost::new(&[(HeadlessMod::UPDATE, 0x1000), (HeadlessMod::TICK, 0x2000)]);

    let result = HookInstaller::install(&host, &runtime(), &HeadlessMod::hooks());

    assert!(result.is_ok());
    assert_eq!(host.hooked(), [HeadlessMod::UPDATE, HeadlessMod::TICK]);
    assert!(host.warnings().is_empty());
}

#[test]
fn a_missing_tick_entry_fails_enable_with_no_hook_left() {
    let mut entries = required_entries();
    entries.retain(|(id, _)| *id != HeadlessMod::TICK);
    let host = FakeHost::new(&entries);

    let result = HeadlessMod::enable_with(&host, &runtime());

    assert!(result.is_err());
    assert!(host.hooked().is_empty());
}

#[test]
fn missing_optional_reads_turn_off_only_their_capabilities() {
    let host = FakeHost::new(&required_entries());
    let runtime = runtime();

    let result = HeadlessMod::enable_with(&host, &runtime);

    assert!(result.is_ok());
    assert_eq!(host.hooked(), [HeadlessMod::UPDATE, HeadlessMod::TICK]);
    assert_eq!(runtime.capabilities(), Capabilities::new(false, false, false, false, false, false));
    let warnings = host.warnings();
    for feature in [
        HeadlessEntry::POSITION,
        HeadlessEntry::ROTATION,
        HeadlessEntry::DIMENSION,
        HeadlessEntry::JOIN,
        HeadlessEntry::LEAVE,
        HeadlessEntry::SERVERS_TAB,
        HeadlessEntry::COMMAND,
    ] {
        assert_eq!(warnings.iter().filter(|line| line.starts_with(feature)).count(), 1, "{warnings:?}");
    }
}

#[test]
fn missing_update_entry_fails_enable() {
    let host = FakeHost::new(&[]);

    let result = HeadlessMod::enable_with(&host, &runtime());

    assert!(result.is_err());
    assert!(host.hooked().is_empty());
}
