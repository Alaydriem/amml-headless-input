use alaydriem_headless_input::HeadlessMod;
use alaydriem_headless_input::game::HeadlessEntry;
use alaydriem_headless_input::services::HostResolver;
use amml_game::GameEntries;
use headless_input_core::structs::Capabilities;

use crate::support::fake_host::FakeHost;

fn capabilities_without(missing: &[&str]) -> Capabilities {
    let present: Vec<(&'static str, u64)> = HeadlessMod::wanted()
        .iter()
        .map(|(id, _)| id.as_str())
        .filter(|id| !missing.contains(id))
        .map(|id| (id, 8))
        .collect();
    let host = FakeHost::new(&present);
    let entries = GameEntries::resolve(&HostResolver(&host), &HeadlessMod::wanted()).unwrap();
    HeadlessEntry::capabilities(&entries)
}

#[test]
fn every_read_entry_gives_every_capability() {
    assert_eq!(capabilities_without(&[]), Capabilities::all());
}

#[test]
fn a_missing_read_entry_turns_off_only_its_capability() {
    assert_eq!(
        capabilities_without(&["StateVectorComponent.mPos"]),
        Capabilities::new(false, true, true, true, true, true)
    );
    assert_eq!(
        capabilities_without(&["ActorRotationComponent.mRot"]),
        Capabilities::new(true, false, true, true, true, true)
    );
    assert_eq!(
        capabilities_without(&["Dimension.getDimensionId"]),
        Capabilities::new(true, true, false, true, true, true)
    );
}

#[test]
fn a_missing_join_entry_turns_off_only_the_join() {
    for entry in [
        "ServerWorldJoiner.joinExternalServer",
        "MinecraftGame.getWorldSystem",
        "WorldSystem.mNetworkWorldJoiner",
        "NetworkWorldJoiner.mServerWorldJoiner",
        "ClientInstance.getMinecraftGame",
        "MinecraftGame.isPrimaryClientInstanceReady",
    ] {
        assert_eq!(capabilities_without(&[entry]), Capabilities::new(true, true, true, false, true, true), "{entry}");
    }
    assert_eq!(
        capabilities_without(&["ClientInstance.requestLeaveGame"]),
        Capabilities::new(true, true, true, true, false, true)
    );
}

#[test]
fn a_missing_servers_tab_entry_leaves_the_join_feature_on() {
    for entry in ["ClientInstance.getCurrentUIRouter", "OreUIRouter.pushRoute"] {
        assert_eq!(capabilities_without(&[entry]), Capabilities::all(), "{entry}");
    }
}

#[test]
fn a_missing_command_entry_turns_off_only_sending_a_command() {
    for entry in [
        "MinecraftPackets.createPacket",
        "ClientInstance.getPacketSender",
        "PacketSender.send",
        "CommandRequestPacket.mCommand",
        "CommandRequestPacket.size",
    ] {
        assert_eq!(capabilities_without(&[entry]), Capabilities::new(true, true, true, true, true, false), "{entry}");
    }
}
