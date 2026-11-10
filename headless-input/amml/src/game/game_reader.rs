use std::ffi::c_void;

use amml_game::{Actor, ClientInstance, EntryId, GameEntries, GameHeap, RawObject};
use amml_msvc::{MemberCall, MsvcString};
use headless_input_core::player::{PlayerView, ScreenKind};
use headless_input_core::structs::Capabilities;

use crate::game::{GameText, HeadlessEntry, ScreenReader};
use crate::types::FrameView;

// Reads the local player and the top screen out of game memory. A read whose capability is off leaves its field at
// zero (the dimension unknown); a read whose capability is on and that meets a null pointer gives no view. Game
// thread
pub struct GameReader<'a> {
    entries: &'a GameEntries,
    capabilities: Capabilities,
}

impl<'a> GameReader<'a> {
    pub fn new(entries: &'a GameEntries, capabilities: Capabilities) -> Self {
        Self { entries, capabilities }
    }

    /// The view of the local player on this tick; `None` without a local player, for a client that is not a
    /// `ClientInstance`, or when a read that is on meets a null pointer.
    ///
    /// # Safety
    /// `client` is 0 or the address of a live `ClientInstance` of the build the entries were resolved for, and the
    /// call is on the game thread.
    #[allow(unsafe_code)]
    pub unsafe fn read(&self, client: usize) -> Option<PlayerView> {
        // SAFETY: forwarded to the caller
        let client = unsafe { self.client(client) }?;
        let player = client.local_player()?;
        let mut view = Self::empty(self.screen(&client, &ScreenKind::InWorld));
        self.fill(&player, &mut view)?;
        Some(view)
    }

    /// What this frame shows: the top screen and, when there is a local player, its view; `None` for a client that
    /// is not a `ClientInstance`. `in_world` and `view.screen` come from the same screen read. A toast on top gives
    /// `previous`, the screen of the last frame.
    ///
    /// # Safety
    /// As [`GameReader::read`].
    #[allow(unsafe_code)]
    pub unsafe fn frame(&self, client: usize, previous: &ScreenKind) -> Option<FrameView> {
        // SAFETY: forwarded to the caller
        let client = unsafe { self.client(client) }?;
        let screen = self.screen(&client, previous);
        let player = client.local_player();
        let in_world = screen == ScreenKind::InWorld || player.is_some();
        let mut view = Self::empty(screen);
        let mut fresh = false;
        if let Some(player) = &player {
            fresh = self.fill(player, &mut view).is_some();
            if !fresh {
                view = Self::empty(view.screen);
            }
        }
        Some(FrameView { view, in_world, fresh })
    }

    fn empty(screen: ScreenKind) -> PlayerView {
        PlayerView { position: [0.0; 3], yaw: 0.0, pitch: 0.0, dimension: None, screen }
    }

    /// The client at `client`, when it is not null and its first qword is the ClientInstance vtable.
    ///
    /// # Safety
    /// As [`GameReader::read`], and the client stays live while the wrapper is used.
    #[allow(unsafe_code)]
    pub unsafe fn client(&self, client: usize) -> Option<ClientInstance<'a>> {
        let table = self.entries.get(HeadlessEntry::CLIENT_INSTANCE_VTABLE)?;
        let pointer = client as *mut c_void;
        if pointer.is_null() {
            return None;
        }
        // SAFETY: the caller guarantees a live ClientInstance, whose first qword is readable
        let vtable = unsafe { RawObject::read::<u64>(pointer, 0) }?;
        if vtable != table {
            return None;
        }
        // SAFETY: the pointer is a live ClientInstance (guaranteed by the caller) of the build the entries were
        // resolved for
        unsafe { ClientInstance::from_raw(pointer, self.entries) }
    }

    // the top screen, `previous` under a toast; `Other("")` when the slot is null or there is no game heap to free a
    // long name with, in which case the game is not called
    #[allow(unsafe_code)]
    fn screen(&self, client: &ClientInstance<'_>, previous: &ScreenKind) -> ScreenKind {
        let unknown = || ScreenKind::Other(String::new());
        let Some(slot) = self.entries.get(HeadlessEntry::CLIENT_INSTANCE_GET_TOP_SCREEN_NAME) else {
            return unknown();
        };
        let Some(heap) = GameHeap::msvc() else {
            return unknown();
        };
        // SAFETY: the wrapper holds a live ClientInstance, whose first qword is its vtable, and the entry is the slot
        // of `getTopScreenName` in it
        let Some(function) = (unsafe { RawObject::slot(client.as_ptr(), slot) }) else {
            return unknown();
        };
        // SAFETY: the slot holds `std::string IClientInstance::getTopScreenName() const`, which returns the string
        // through a hidden pointer, and the client is live
        let mut name: MsvcString = unsafe { MemberCall::call_sret0(function, client.as_ptr()) };
        let kind = ScreenReader::kind(&GameText::text(&name), previous, client.local_player().is_some());
        // SAFETY: the game allocated the string's block, if it has one, with the allocator `heap` mirrors; the string
        // is not used again
        unsafe { name.release(&heap) };
        kind
    }

    // the position, angles and dimension whose capabilities are on; None when one of them meets a null pointer
    fn fill(&self, player: &Actor<'_>, view: &mut PlayerView) -> Option<()> {
        if self.capabilities.position {
            view.position = self.position(player)?;
        }
        if self.capabilities.rotation {
            let [pitch, yaw] = self.rotation(player)?;
            view.pitch = pitch;
            view.yaw = yaw;
        }
        if self.capabilities.dimension {
            view.dimension = Some(player.dimension()?.id()?);
        }
        Some(())
    }

    // the address of the player's StateVectorComponent
    #[allow(unsafe_code)]
    fn state_vector(&self, player: &Actor<'_>) -> Option<*mut c_void> {
        let offset = self.entries.get(EntryId::ACTOR_STATE_VECTOR_COMPONENT)?;
        // SAFETY: the wrapper holds a live Actor, and the entry is the offset of its StateVectorComponent pointer
        unsafe { RawObject::pointer(player.as_ptr(), offset) }
    }

    // `StateVectorComponent::mPos`, a Vec3
    #[allow(unsafe_code)]
    fn position(&self, player: &Actor<'_>) -> Option<[f32; 3]> {
        let pos_at = self.entries.get(EntryId::STATE_VECTOR_COMPONENT_POS)?;
        let component = self.state_vector(player)?;
        // SAFETY: a non-null component pointer of a live actor points to its StateVectorComponent, which holds a Vec3
        // at `pos_at`
        unsafe { RawObject::read::<[f32; 3]>(component, pos_at) }
    }

    // `ActorRotationComponent::mRot`, a Vec2 with the pitch in x and the yaw in y
    #[allow(unsafe_code)]
    fn rotation(&self, player: &Actor<'_>) -> Option<[f32; 2]> {
        let component_at = self.entries.get(EntryId::ACTOR_ACTOR_ROTATION_COMPONENT)?;
        let rot_at = self.entries.get(EntryId::ACTOR_ROTATION_COMPONENT_ROT)?;
        // SAFETY: the wrapper holds a live Actor, and the entry is the offset of its ActorRotationComponent pointer
        let component = unsafe { RawObject::pointer(player.as_ptr(), component_at) }?;
        // SAFETY: a non-null component pointer of a live actor points to its ActorRotationComponent, which holds a
        // Vec2 at `rot_at`
        unsafe { RawObject::read::<[f32; 2]>(component, rot_at) }
    }
}
