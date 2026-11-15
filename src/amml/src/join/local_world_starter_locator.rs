use std::ffi::c_void;

use amml_game::RawObject;

// Finds the game's `World::LocalWorldStarter`: the MinecraftGame getter in a vtable slot gives the world system, which
// holds the starter pointer at a field offset. Every step is a game read; a null at any step gives None, never a call
// on a null object
pub struct LocalWorldStarterLocator;

impl LocalWorldStarterLocator {
    /// The `LocalWorldStarter` address, or None when a step finds null.
    ///
    /// # Safety
    /// `game` is null or a live `MinecraftGame` whose vtable holds the owner getter at `owner_slot`, and `starter_at` is
    /// the offset in the owner of the `LocalWorldStarter` pointer. The call is on the game thread.
    #[allow(unsafe_code)]
    pub unsafe fn locate(game: *mut c_void, owner_slot: u64, starter_at: u64) -> Option<*mut c_void> {
        if game.is_null() {
            return None;
        }
        // SAFETY: the caller guarantees `game` is live with a vtable that holds `owner_slot`
        let getter = unsafe { RawObject::slot(game, owner_slot) }?;
        // SAFETY: the slot holds `X* MinecraftGame::getter() const`, and `game` is live
        let owner = unsafe { RawObject::call_pointer(getter, game) }?;
        // SAFETY: the caller guarantees the owner holds the pointer at `starter_at`
        unsafe { RawObject::pointer(owner, starter_at) }
    }
}
