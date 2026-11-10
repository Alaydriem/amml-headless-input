use amml_game::{EntryId, GameEntries};
use headless_input_core::structs::Capabilities;

// The game entries the reads use that `amml_game::EntryId` has no constant for, the features the optional ones
// serve, and the capabilities a set of resolved entries gives
pub struct HeadlessEntry;

impl HeadlessEntry {
    // the address of the ClientInstance vtable: a client pointer whose first qword differs is not read
    pub const CLIENT_INSTANCE_VTABLE: EntryId = EntryId::new("ClientInstance.vtable");
    // the slot of `getTopScreenName` in it
    pub const CLIENT_INSTANCE_GET_TOP_SCREEN_NAME: EntryId = EntryId::new("ClientInstance.getTopScreenName");
    // the offset of the GameCallbacks subobject in a ClientInstance: the client is the subobject address minus this
    pub const CLIENT_INSTANCE_GAME_CALLBACKS_OFFSET: EntryId = EntryId::new("ClientInstance.gameCallbacksOffset");
    // the game's own host and port join: `World::ServerWorldJoiner::joinExternalServer`, the MinecraftGame vslot of the
    // object that owns the NetworkWorldJoiner, the field of that owner that holds the NetworkWorldJoiner, and the
    // ServerWorldJoiner member of it
    pub const SERVER_WORLD_JOINER_JOIN_EXTERNAL_SERVER: EntryId = EntryId::new("ServerWorldJoiner.joinExternalServer");
    pub const MINECRAFT_GAME_GET_SERVER_WORLD_JOINER_OWNER: EntryId = EntryId::new("MinecraftGame.getWorldSystem");
    pub const NETWORK_WORLD_JOINER_OWNER_NETWORK_WORLD_JOINER: EntryId =
        EntryId::new("WorldSystem.mNetworkWorldJoiner");
    pub const NETWORK_WORLD_JOINER_SERVER_WORLD_JOINER: EntryId = EntryId::new("NetworkWorldJoiner.mServerWorldJoiner");

    // the Play > Servers tab: the slot of `getCurrentUIRouter` in the ClientInstance vtable, and the pushRoute function of the
    // navigator it returns
    pub const CLIENT_INSTANCE_GET_CURRENT_UI_ROUTER: EntryId = EntryId::new("ClientInstance.getCurrentUIRouter");
    pub const ORE_UI_ROUTER_PUSH_ROUTE: EntryId = EntryId::new("OreUIRouter.pushRoute");
    // the Ore UI route of the Servers tab
    pub const SERVERS_ROUTE: &'static str = "/play/servers";
    pub const SERVERS_TAB: &'static str = "the servers tab";

    // sending a command: the packet factory, the slot of `getPacketSender` in the ClientInstance vtable, the slot of
    // `send` in the PacketSender vtable, and the offset of `mCommand` in a CommandRequestPacket with its size
    pub const MINECRAFT_PACKETS_CREATE_PACKET: EntryId = EntryId::new("MinecraftPackets.createPacket");
    pub const CLIENT_INSTANCE_GET_PACKET_SENDER: EntryId = EntryId::new("ClientInstance.getPacketSender");
    pub const PACKET_SENDER_SEND: EntryId = EntryId::new("PacketSender.send");
    pub const COMMAND_REQUEST_PACKET_M_COMMAND: EntryId = EntryId::new("CommandRequestPacket.mCommand");
    pub const COMMAND_REQUEST_PACKET_SIZE: EntryId = EntryId::new("CommandRequestPacket.size");

    pub const POSITION: &'static str = "the player position";
    pub const ROTATION: &'static str = "the player rotation";
    pub const DIMENSION: &'static str = "the dimension";
    pub const JOIN: &'static str = "the server join";
    pub const LEAVE: &'static str = "the world leave";
    pub const COMMAND: &'static str = "sending a command";

    pub const POSITION_ENTRIES: [EntryId; 2] =
        [EntryId::ACTOR_STATE_VECTOR_COMPONENT, EntryId::STATE_VECTOR_COMPONENT_POS];
    // the look is read from the rotation component; turning goes through the manager
    pub const ROTATION_ENTRIES: [EntryId; 2] =
        [EntryId::ACTOR_ACTOR_ROTATION_COMPONENT, EntryId::ACTOR_ROTATION_COMPONENT_ROT];
    pub const DIMENSION_ENTRIES: [EntryId; 2] = [EntryId::ACTOR_GET_DIMENSION, EntryId::DIMENSION_GET_DIMENSION_ID];
    // the game's host and port join, the chain to it, the game it starts from, and the game it asks whether the primary
    // client is ready
    pub const JOIN_ENTRIES: [EntryId; 6] = [
        Self::SERVER_WORLD_JOINER_JOIN_EXTERNAL_SERVER,
        Self::MINECRAFT_GAME_GET_SERVER_WORLD_JOINER_OWNER,
        Self::NETWORK_WORLD_JOINER_OWNER_NETWORK_WORLD_JOINER,
        Self::NETWORK_WORLD_JOINER_SERVER_WORLD_JOINER,
        EntryId::CLIENT_INSTANCE_GET_MINECRAFT_GAME,
        EntryId::MINECRAFT_GAME_IS_PRIMARY_CLIENT_INSTANCE_READY,
    ];
    pub const LEAVE_ENTRIES: [EntryId; 1] = [EntryId::CLIENT_INSTANCE_REQUEST_LEAVE_GAME];
    // the packet route of a command: the factory, the sender and its send, and the packet layout
    pub const COMMAND_ENTRIES: [EntryId; 5] = [
        Self::MINECRAFT_PACKETS_CREATE_PACKET,
        Self::CLIENT_INSTANCE_GET_PACKET_SENDER,
        Self::PACKET_SENDER_SEND,
        Self::COMMAND_REQUEST_PACKET_M_COMMAND,
        Self::COMMAND_REQUEST_PACKET_SIZE,
    ];

    // a capability is on when every entry it needs resolved
    pub fn capabilities(entries: &GameEntries) -> Capabilities {
        let all = |ids: &[EntryId]| ids.iter().all(|id| entries.get(*id).is_some());
        Capabilities::new(
            all(&Self::POSITION_ENTRIES),
            all(&Self::ROTATION_ENTRIES),
            all(&Self::DIMENSION_ENTRIES),
            all(&Self::JOIN_ENTRIES),
            all(&Self::LEAVE_ENTRIES),
            all(&Self::COMMAND_ENTRIES),
        )
    }
}
