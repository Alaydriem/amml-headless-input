use alaydriem_headless_input::services::{HeadlessDescriptor, SettingsReader};
use amm_core::services::DescriptorChecker;
use amm_core::services::settings::SchemaChecker;
use amm_core::structs::descriptor::{Capability, ClaimMode};
use amm_core::structs::settings::HclDocument;
use headless_input_core::config::HeadlessConfig;

#[test]
fn the_descriptor_holds_a_full_block_input_claim() {
    let d = DescriptorChecker::new(128, 256).parse(HeadlessDescriptor::TEXT).unwrap();

    assert_eq!(d.id(), "@alaydriem/headless-input");
    assert_eq!(d.claims.len(), 1);
    let claim = &d.claims[0];
    assert_eq!(claim.capability, Capability::Input);
    assert_eq!(claim.mode, ClaimMode::Exclusive);
    assert!(claim.full_block);
}

#[test]
fn schema_defaults_match_the_config_defaults() {
    let d = DescriptorChecker::new(128, 256).parse(HeadlessDescriptor::TEXT).unwrap();
    let checked = SchemaChecker::check(&d.settings, HclDocument::empty());

    assert_eq!(SettingsReader::read(&checked.document.text()), HeadlessConfig::default());
}
