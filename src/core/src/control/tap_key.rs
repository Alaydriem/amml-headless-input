use amm_core::services::settings::KeyNames;

use crate::errors::TapKeyError;

// One key that `press` taps: down on one frame, up on the next. It holds the Windows virtual-key code
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TapKey {
    vk: u32,
}

impl TapKey {
    const ESC_ALIAS: &'static str = "Esc";
    const ESCAPE: &'static str = "Escape";
    // the manager takes key codes 1 to 255 only
    const MAX_VK: u32 = 0xFF;
    // the keys other tools hold or tap, with the tool that drives each; a tap of one would fight that tool
    const OWNED: [(u32, &'static str); 16] = [
        (0x57, "move"),
        (0x41, "move"),
        (0x53, "move"),
        (0x44, "move"),
        (0x20, "jump"),
        (0x10, "sneak"),
        (0x11, "sprint"),
        (0x31, "hotbar"),
        (0x32, "hotbar"),
        (0x33, "hotbar"),
        (0x34, "hotbar"),
        (0x35, "hotbar"),
        (0x36, "hotbar"),
        (0x37, "hotbar"),
        (0x38, "hotbar"),
        (0x39, "hotbar"),
    ];

    // the key for a name of the manager's key table (case ignored, `VK_0x01` to `VK_0xFF` for unnamed keys), or `Esc`; a key
    // that another tool drives is refused
    pub fn parse(name: &str) -> Result<Self, TapKeyError> {
        let trimmed = name.trim();
        let lookup = if trimmed.eq_ignore_ascii_case(Self::ESC_ALIAS) { Self::ESCAPE } else { trimmed };
        let vk = KeyNames::vk(lookup)
            .filter(|vk| (1..=Self::MAX_VK).contains(vk))
            .ok_or_else(|| TapKeyError::Unknown(name.to_owned()))?;
        if let Some((_, tool)) = Self::OWNED.iter().find(|(code, _)| *code == vk) {
            return Err(TapKeyError::Owned { name: name.to_owned(), tool });
        }
        Ok(Self { vk })
    }

    pub fn vk(self) -> u32 {
        self.vk
    }
}
