use std::ffi::c_void;

use amml_game::RawObject;

// Finds the game's `World::ServerWorldJoiner`: the MinecraftGame getter in a vtable slot gives an owner object, the
// owner holds the `NetworkWorldJoiner` pointer at a field offset, and the `ServerWorldJoiner` is a member of that at
// another offset. Every step is a game read; a null at any step gives None, never a call on a null object
pub struct ServerJoinerLocator;

impl ServerJoinerLocator {
    /// The `ServerWorldJoiner` address, or None when a step finds null or an offset does not fit.
    ///
    /// # Safety
    /// `game` is null or a live `MinecraftGame` whose vtable holds the owner getter at `owner_slot`, `joiner_at` is the
    /// offset in the owner of the `NetworkWorldJoiner` pointer, and `member_at` the offset in the `NetworkWorldJoiner`
    /// of the `ServerWorldJoiner`. The call is on the game thread.
    #[allow(unsafe_code)]
    pub unsafe fn locate(game: *mut c_void, owner_slot: u64, joiner_at: u64, member_at: u64) -> Option<*mut c_void> {
        if game.is_null() {
            return None;
        }
        // SAFETY: the caller guarantees `game` is live with a vtable that holds `owner_slot`
        let getter = unsafe { RawObject::slot(game, owner_slot) }?;
        // SAFETY: the slot holds `X* MinecraftGame::getter() const`, and `game` is live
        let owner = unsafe { RawObject::call_pointer(getter, game) }?;
        // SAFETY: the caller guarantees the owner holds the pointer at `joiner_at`
        let joiner = unsafe { RawObject::pointer(owner, joiner_at) }?;
        let member_at = usize::try_from(member_at).ok()?;
        Some(joiner.cast::<u8>().wrapping_add(member_at).cast())
    }
}
