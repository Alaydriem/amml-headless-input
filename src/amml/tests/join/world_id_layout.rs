use amml_msvc::MsvcString;

// the game's `World::WorldID` is one std::string at offset 0; the world join passes an MsvcString as it
#[test]
fn the_world_id_argument_is_one_msvc_string_of_0x20_bytes() {
    assert_eq!(size_of::<MsvcString>(), 0x20);
}
