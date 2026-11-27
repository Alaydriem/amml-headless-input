use alaydriem_headless_input::HeadlessMod;
use amm_client::link::LinkUses;
use amml_api::AmmlMod;

#[test]
fn module_matches_namespace_and_name() {
    assert_eq!(HeadlessMod::NAMESPACE, "alaydriem");
    assert_eq!(HeadlessMod::NAME, "headless-input");
    assert_eq!(HeadlessMod::MODULE, format!("@{}/{}", HeadlessMod::NAMESPACE, HeadlessMod::NAME));
    assert_eq!(HeadlessMod::id(), HeadlessMod::MODULE);
}

#[test]
fn the_uses_hold_every_row_the_manager_link_needs() {
    assert!(LinkUses::check(&HeadlessMod::USES).is_ok());
}

#[test]
fn every_row_of_the_uses_is_required() {
    assert!(HeadlessMod::USES.rows().iter().all(|row| row.required != 0));
}

#[test]
fn the_manifest_uses_block_is_the_uses_of_the_mod() {
    let text = include_str!("../amml.hcl").replace("\r\n", "\n");
    assert_eq!(HeadlessMod::USES.write_into(&text).replace("\r\n", "\n"), text);
}
