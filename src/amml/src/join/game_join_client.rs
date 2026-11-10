use std::ffi::c_void;

use amml_game::{ClientInstance, EntryId, GameEntries, GameHeap, RawObject};
use amml_msvc::{MemberCall, MsvcString};
use headless_input_core::structs::JoinTarget;

use crate::game::{GameStrings, GameText, HeadlessEntry};
use crate::join::{JoinClient, JoinResultCallback, ServerJoinerLocator};

// The game's client behind the `JoinClient` trait: each call reads the code pointer of its entry and calls it. A call
// whose entry is off does nothing and answers no. Game thread
pub struct GameJoinClient<'a> {
    client: ClientInstance<'a>,
    entries: &'a GameEntries,
}

impl<'a> GameJoinClient<'a> {
    pub fn new(client: ClientInstance<'a>, entries: &'a GameEntries) -> Self {
        Self { client, entries }
    }

    // a function of the client's own vtable
    #[allow(unsafe_code)]
    fn client_function(&self, id: EntryId) -> Option<*const c_void> {
        let slot = self.entries.get(id)?;
        // SAFETY: the wrapper holds a live ClientInstance, whose first qword is its vtable, and `id` is a slot entry
        // of it
        unsafe { RawObject::slot(self.client.as_ptr(), slot) }
    }

    // the game's ServerWorldJoiner; None when an entry is off or a step of the chain is null
    #[allow(unsafe_code)]
    fn joiner(&self) -> Option<*mut c_void> {
        let owner_slot = self.entries.get(HeadlessEntry::MINECRAFT_GAME_GET_SERVER_WORLD_JOINER_OWNER)?;
        let joiner_at = self.entries.get(HeadlessEntry::NETWORK_WORLD_JOINER_OWNER_NETWORK_WORLD_JOINER)?;
        let member_at = self.entries.get(HeadlessEntry::NETWORK_WORLD_JOINER_SERVER_WORLD_JOINER)?;
        let game = self.client.minecraft_game()?;
        // SAFETY: the client returned a non-null IMinecraftGame*, a live MinecraftGame, and the entries describe the
        // chain from it to the joiner. The call is on the game thread
        unsafe { ServerJoinerLocator::locate(game, owner_slot, joiner_at, member_at) }
    }
}

impl JoinClient for GameJoinClient<'_> {
    #[allow(unsafe_code)]
    fn is_primary_ready(&self) -> bool {
        let Some(slot) = self.entries.get(EntryId::MINECRAFT_GAME_IS_PRIMARY_CLIENT_INSTANCE_READY) else {
            return false;
        };
        let Some(game) = self.client.minecraft_game() else {
            return false;
        };
        // SAFETY: the client returned a non-null IMinecraftGame*, a live MinecraftGame whose first qword is the
        // MinecraftGame vtable, and the entry is a slot of it
        let Some(function) = (unsafe { RawObject::slot(game, slot) }) else {
            return false;
        };
        // SAFETY: the slot holds `bool IMinecraftGame::isPrimaryClientInstanceReady() const`, and the game is live
        unsafe { RawObject::call_bool(function, game) }
    }

    // a client with a local player is in a world
    fn is_in_game(&self) -> bool {
        self.client.local_player().is_some()
    }

    // empty when the entry is off or there is no game heap to free a long name with; the game is then not called
    #[allow(unsafe_code)]
    fn top_screen_name(&self) -> String {
        let Some(function) = self.client_function(HeadlessEntry::CLIENT_INSTANCE_GET_TOP_SCREEN_NAME) else {
            return String::new();
        };
        let Some(heap) = GameHeap::msvc() else {
            return String::new();
        };
        // SAFETY: the slot holds `std::string IClientInstance::getTopScreenName() const`, which returns the string
        // through a hidden pointer, and the client is live
        let mut name: MsvcString = unsafe { MemberCall::call_sret0(function, self.client.as_ptr()) };
        let text = GameText::text(&name).into_owned();
        // SAFETY: the game allocated the string's block, if it has one, with the allocator `heap` mirrors; the string
        // is not used again
        unsafe { name.release(&heap) };
        text
    }

    #[allow(unsafe_code)]
    fn request_leave_game(&self) {
        let Some(function) = self.client_function(EntryId::CLIENT_INSTANCE_REQUEST_LEAVE_GAME) else {
            return;
        };
        // SAFETY: the slot holds `void ClientInstance::requestLeaveGame(bool, bool)`: this in rcx, the two bools in
        // dl and r8b
        let call: unsafe extern "C" fn(*mut c_void, bool, bool) = unsafe { std::mem::transmute(function) };
        // SAFETY: the client is live and the signature is as above
        unsafe { call(self.client.as_ptr(), true, false) };
    }

    // the game's own call for the Servers tab: the router comes from the client's `getCurrentUIRouter` slot, and the
    // route goes in as a std::string with mode 0
    #[allow(unsafe_code)]
    fn open_servers_tab(&self) -> bool {
        if self.client.local_player().is_some() {
            return false;
        }
        let Some(navigate) = self.entries.get(HeadlessEntry::ORE_UI_ROUTER_PUSH_ROUTE) else {
            return false;
        };
        let Some(getter) = self.client_function(HeadlessEntry::CLIENT_INSTANCE_GET_CURRENT_UI_ROUTER) else {
            return false;
        };
        // an address of 0 is no function: never made into a function pointer
        let Ok(navigate) = usize::try_from(navigate) else {
            return false;
        };
        if navigate == 0 {
            return false;
        }
        let Some(heap) = GameHeap::msvc() else {
            return false;
        };
        // SAFETY: the slot holds `OreUI::Router& IClientInstance::getCurrentUIRouter()`, and the client is live
        let Some(navigator) = (unsafe { RawObject::call_pointer(getter, self.client.as_ptr()) }) else {
            return false;
        };
        let Some(mut route) = GameStrings::heap_string(&heap, HeadlessEntry::SERVERS_ROUTE) else {
            return false;
        };
        // SAFETY: the entry is `bool OreUI::Router::_pushRoute(const std::string& route, RouterPushMode mode)`: this in rcx, the
        // route in rdx and the mode in r8d. The game copies the route, so the string stays ours
        let call: unsafe extern "C" fn(*mut c_void, *const MsvcString, i32) -> u8 =
            unsafe { std::mem::transmute(navigate) };
        // SAFETY: the navigator is live and on the game thread (the caller), and the route string is live
        let accepted = unsafe { call(navigator, &raw const route, 0) };
        // SAFETY: the string came from `GameStrings::heap_string` with this heap and is not used again; the call copied it
        unsafe { route.release(&heap) };
        accepted != 0
    }

    fn take_join_result(&self) -> Option<i32> {
        JoinResultCallback::take_result()
    }

    fn clear_join_result(&self) {
        JoinResultCallback::clear_result();
    }

    // the game's own host and port join, as the party travel code calls it: the game resolves the host, tests the
    // connection and joins; the answer arrives later through the callback. A null step of the chain to the joiner gives
    // no call
    #[allow(unsafe_code)]
    fn join_server(&self, target: &JoinTarget) -> bool {
        let Some(joiner) = self.joiner() else {
            return false;
        };
        let Some(function) = self.entries.get(HeadlessEntry::SERVER_WORLD_JOINER_JOIN_EXTERNAL_SERVER) else {
            return false;
        };
        // an address of 0 is no function: never made into a function pointer
        let Ok(function) = usize::try_from(function) else {
            return false;
        };
        if function == 0 {
            return false;
        }
        let Some(heap) = GameHeap::msvc() else {
            return false;
        };
        let Some(mut host_text) = GameStrings::heap_string(&heap, &target.host) else {
            return false;
        };
        let Some(mut name_text) = GameStrings::heap_string(&heap, &target.host) else {
            // SAFETY: the string came from `GameStrings::heap_string` with this heap and is not used again
            unsafe { host_text.release(&heap) };
            return false;
        };
        // the callback is passed by value: the game moves it out and destroys what is left, and the block is ours
        let mut callback = JoinResultCallback::function();
        // SAFETY: the entry is `void World::ServerWorldJoiner::joinExternalServer(const std::string& host, unsigned
        // short port, const std::string& name, std::function<void(JoinServerWorldResult)> onResult)`: this in rcx,
        // the host in rdx, the port in r8w, the name in r9 and a pointer to the by-value std::function on the stack.
        // The game copies both strings, so they stay ours
        let join: unsafe extern "C" fn(*mut c_void, *const MsvcString, u16, *const MsvcString, *mut [*mut c_void; 8]) =
            unsafe { std::mem::transmute(function) };
        // SAFETY: the joiner is live and on the game thread (the caller), the strings are live, and the callback block
        // holds a callable the game can copy, move, call and delete
        unsafe { join(joiner, &raw const host_text, target.port, &raw const name_text, &raw mut callback) };
        // SAFETY: both strings came from `GameStrings::heap_string` with this heap and are not used again; the join call copied
        // them
        unsafe {
            host_text.release(&heap);
            name_text.release(&heap);
        }
        true
    }
}
