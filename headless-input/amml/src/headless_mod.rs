use std::ffi::c_void;

use amml_game::{EntryId, EntryNeed, GameEntries, GameError};
use amml_sdk::{AmmlMod, Api, ModDirs, SdkError};

use crate::detours::{client_on_tick, client_update};
use crate::game::HeadlessEntry;
use crate::services::{AmmlHost, HeadlessRuntime, HookInstaller, HostResolver};
use crate::traits::GameHost;
use crate::types::HookSpec;

// the AMML entry point: load makes the runtime, enable installs the hooks and starts the MCP side, disable detaches
// (input released, the lease ended, the hub and Tokio stopped) and then removes the hooks. The manager link is made on
// the first ClientInstance::update after enable
pub struct HeadlessMod;

impl HeadlessMod {
    // the id as the module string the loader and the hook host take; a test checks it against `AmmlMod::id`
    pub const MODULE: &'static str = "@alaydriem/headless-input";
    pub const UPDATE: &'static str = "ClientInstance.update";
    pub const TICK: &'static str = "ClientInstance.onTick";

    // every hook, in install order. Without the client tick no run advances, so it is required
    pub fn hooks() -> [HookSpec; 2] {
        [
            HookSpec {
                entry: Self::UPDATE,
                detour: client_update::detour as *const c_void,
                original: client_update::GUARD.original(),
                required: true,
                priority: 0,
                feature: "the frame",
            },
            HookSpec {
                entry: Self::TICK,
                detour: client_on_tick::detour as *const c_void,
                original: client_on_tick::GUARD.original(),
                required: true,
                priority: 0,
                feature: "the client tick",
            },
        ]
    }

    // the game reads beyond the hooks. `ClientInstance.gameCallbacksOffset` is required: the client tick finds its
    // client through it. A missing optional entry turns off only its capability
    pub fn wanted() -> [(EntryId, EntryNeed); 24] {
        const POSITION: EntryNeed = EntryNeed::Optional { feature: HeadlessEntry::POSITION };
        const ROTATION: EntryNeed = EntryNeed::Optional { feature: HeadlessEntry::ROTATION };
        const DIMENSION: EntryNeed = EntryNeed::Optional { feature: HeadlessEntry::DIMENSION };
        const JOIN: EntryNeed = EntryNeed::Optional { feature: HeadlessEntry::JOIN };
        const TAB: EntryNeed = EntryNeed::Optional { feature: HeadlessEntry::SERVERS_TAB };
        const LEAVE: EntryNeed = EntryNeed::Optional { feature: HeadlessEntry::LEAVE };
        const COMMAND: EntryNeed = EntryNeed::Optional { feature: HeadlessEntry::COMMAND };
        [
            (EntryId::CLIENT_INSTANCE_GET_LOCAL_PLAYER, EntryNeed::Required),
            (HeadlessEntry::CLIENT_INSTANCE_GET_TOP_SCREEN_NAME, EntryNeed::Required),
            (HeadlessEntry::CLIENT_INSTANCE_VTABLE, EntryNeed::Required),
            (HeadlessEntry::CLIENT_INSTANCE_GAME_CALLBACKS_OFFSET, EntryNeed::Required),
            (EntryId::ACTOR_STATE_VECTOR_COMPONENT, POSITION),
            (EntryId::STATE_VECTOR_COMPONENT_POS, POSITION),
            (EntryId::ACTOR_ACTOR_ROTATION_COMPONENT, ROTATION),
            (EntryId::ACTOR_ROTATION_COMPONENT_ROT, ROTATION),
            (EntryId::ACTOR_GET_DIMENSION, DIMENSION),
            (EntryId::DIMENSION_GET_DIMENSION_ID, DIMENSION),
            (HeadlessEntry::SERVER_WORLD_JOINER_JOIN_EXTERNAL_SERVER, JOIN),
            (HeadlessEntry::MINECRAFT_GAME_GET_SERVER_WORLD_JOINER_OWNER, JOIN),
            (HeadlessEntry::NETWORK_WORLD_JOINER_OWNER_NETWORK_WORLD_JOINER, JOIN),
            (HeadlessEntry::NETWORK_WORLD_JOINER_SERVER_WORLD_JOINER, JOIN),
            (EntryId::CLIENT_INSTANCE_GET_MINECRAFT_GAME, JOIN),
            (EntryId::MINECRAFT_GAME_IS_PRIMARY_CLIENT_INSTANCE_READY, JOIN),
            (EntryId::CLIENT_INSTANCE_REQUEST_LEAVE_GAME, LEAVE),
            (HeadlessEntry::CLIENT_INSTANCE_GET_CURRENT_UI_ROUTER, TAB),
            (HeadlessEntry::ORE_UI_ROUTER_PUSH_ROUTE, TAB),
            (HeadlessEntry::MINECRAFT_PACKETS_CREATE_PACKET, COMMAND),
            (HeadlessEntry::CLIENT_INSTANCE_GET_PACKET_SENDER, COMMAND),
            (HeadlessEntry::PACKET_SENDER_SEND, COMMAND),
            (HeadlessEntry::COMMAND_REQUEST_PACKET_M_COMMAND, COMMAND),
            (HeadlessEntry::COMMAND_REQUEST_PACKET_SIZE, COMMAND),
        ]
    }

    // the body of enable: the game entries and the required hook entries must resolve, then the hooks go in and
    // the next frame reaches the manager
    pub fn enable_with<H: GameHost>(host: &H, runtime: &HeadlessRuntime) -> Result<(), SdkError> {
        let entries = GameEntries::resolve(&HostResolver(host), &Self::wanted()).map_err(|error| match error {
            GameError::MissingEntry { id } => SdkError::MissingEntry(id),
            other => SdkError::Mod(other.to_string()),
        })?;
        HookInstaller::install(host, runtime, &Self::hooks())?;
        for (feature, cause) in entries.off_features() {
            host.warn(&format!("{feature} is off ({cause})"));
        }
        runtime.set_entries(entries);
        runtime.enable_link();
        Ok(())
    }
}

impl AmmlMod for HeadlessMod {
    const NAMESPACE: &'static str = "alaydriem";
    const NAME: &'static str = "headless-input";

    fn load(api: &Api) -> Result<(), SdkError> {
        let dirs = ModDirs::new(api, Self::MODULE)?;
        HeadlessRuntime::install(&dirs);
        Ok(())
    }

    fn enable(api: &Api) -> Result<(), SdkError> {
        let runtime = HeadlessRuntime::get().ok_or_else(|| SdkError::Mod("load did not run".to_string()))?;
        let host = AmmlHost::new(api, Self::MODULE);
        Self::enable_with(&host, runtime)?;
        runtime.start_services();
        Ok(())
    }

    fn disable(api: &Api) {
        let Some(runtime) = HeadlessRuntime::get() else { return };
        runtime.detach();
        HookInstaller::remove_all(&AmmlHost::new(api, Self::MODULE), runtime);
    }
}
