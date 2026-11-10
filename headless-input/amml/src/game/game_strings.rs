use amml_msvc::{MsvcHeap, MsvcString};

// Strings the mod hands to game code.
pub struct GameStrings;

impl GameStrings {
    // a `std::string` whose block, if it has one, comes from the game heap, so the game can free it; None when the
    // heap cannot allocate it. The caller releases it with the same heap or moves it into a game object
    #[allow(unsafe_code)]
    pub fn heap_string(heap: &MsvcHeap, text: &str) -> Option<MsvcString> {
        // SAFETY: the heap is the game's operator new and delete pair
        unsafe { MsvcString::new_in(heap, text) }.ok()
    }
}
