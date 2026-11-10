// A movement key a step can hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputKey {
    Forward,
    Back,
    Left,
    Right,
    Jump,
    Sneak,
    Sprint,
}

impl InputKey {
    pub const ALL: [InputKey; 7] =
        [Self::Forward, Self::Back, Self::Left, Self::Right, Self::Jump, Self::Sneak, Self::Sprint];

    // the key named in a control file
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "forward" => Some(Self::Forward),
            "back" => Some(Self::Back),
            "left" => Some(Self::Left),
            "right" => Some(Self::Right),
            "jump" => Some(Self::Jump),
            "sneak" => Some(Self::Sneak),
            "sprint" => Some(Self::Sprint),
            _ => None,
        }
    }
}
