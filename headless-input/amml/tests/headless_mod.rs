use alaydriem_headless_input::HeadlessMod;
use amml_sdk::AmmlMod;

#[test]
fn module_matches_namespace_and_name() {
    assert_eq!(HeadlessMod::NAMESPACE, "alaydriem");
    assert_eq!(HeadlessMod::NAME, "headless-input");
    assert_eq!(HeadlessMod::MODULE, format!("@{}/{}", HeadlessMod::NAMESPACE, HeadlessMod::NAME));
    assert_eq!(HeadlessMod::id(), HeadlessMod::MODULE);
}

#[test]
fn the_tick_hook_is_required() {
    let required: Vec<_> = HeadlessMod::hooks().iter().filter(|h| h.required).map(|h| h.entry).collect();
    assert_eq!(required, ["ClientInstance.update", "ClientInstance.onTick"]);
}

#[test]
fn amml_hcl_lists_exactly_the_hook_and_wanted_entries() {
    let document: hcl::Value = hcl::from_str(include_str!("../amml.hcl")).unwrap();
    let hcl::Value::Object(map) = document else { panic!("the document is not an object") };
    let Some(hcl::Value::Array(listed)) = map.get("entries") else { panic!("no entries") };
    let mut listed: Vec<&str> = listed.iter().filter_map(hcl::Value::as_str).collect();
    assert_eq!(listed.first().copied(), Some("ClientInstance.update"));
    let mut used: Vec<&str> = HeadlessMod::hooks().iter().map(|h| h.entry).collect();
    used.extend(HeadlessMod::wanted().iter().map(|(id, _)| id.as_str()));
    listed.sort_unstable();
    used.sort_unstable();
    assert_eq!(listed, used);
    let mut expected = vec![
        "ClientInstance.update",
        "ClientInstance.onTick",
        "ClientInstance.gameCallbacksOffset",
        "ClientInstance.getLocalPlayer",
        "ClientInstance.getTopScreenName",
        "ClientInstance.vtable",
        "Actor.stateVectorComponent",
        "StateVectorComponent.mPos",
        "Actor.actorRotationComponent",
        "ActorRotationComponent.mRot",
        "Actor.getDimension",
        "Dimension.getDimensionId",
        "ServerWorldJoiner.joinExternalServer",
        "MinecraftGame.getWorldSystem",
        "WorldSystem.mNetworkWorldJoiner",
        "NetworkWorldJoiner.mServerWorldJoiner",
        "ClientInstance.getMinecraftGame",
        "MinecraftGame.isPrimaryClientInstanceReady",
        "ClientInstance.requestLeaveGame",
        "ClientInstance.getCurrentUIRouter",
        "OreUIRouter.pushRoute",
        "MinecraftPackets.createPacket",
        "ClientInstance.getPacketSender",
        "PacketSender.send",
        "CommandRequestPacket.mCommand",
        "CommandRequestPacket.size",
    ];
    expected.sort_unstable();
    assert_eq!(listed, expected);
}
