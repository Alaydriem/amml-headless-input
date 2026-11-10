use std::borrow::Cow;

use amml_msvc::MsvcString;

// Text the hooks read out of the game's `std::string` values.
pub struct GameText;

impl GameText {
    /// The text of `string`, with bytes that are not UTF-8 replaced; empty for a null pointer.
    ///
    /// # Safety
    /// `string` is null or points to a live `std::string` of the game.
    #[allow(unsafe_code)]
    pub unsafe fn read(string: *const MsvcString) -> String {
        // SAFETY: the caller guarantees a live string or null
        match unsafe { string.as_ref() } {
            Some(string) => Self::text(string).into_owned(),
            None => String::new(),
        }
    }

    // the text of a string the mod holds; borrowed, with no copy, when the bytes are UTF-8
    pub fn text(string: &MsvcString) -> Cow<'_, str> {
        String::from_utf8_lossy(string.as_bytes())
    }
}
