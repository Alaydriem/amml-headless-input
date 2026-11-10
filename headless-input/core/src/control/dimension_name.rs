// A dimension a walk can end in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DimensionName {
    Overworld,
    Nether,
    End,
}

impl DimensionName {
    // the game dimension id
    pub fn id(self) -> i32 {
        match self {
            Self::Overworld => 0,
            Self::Nether => 1,
            Self::End => 2,
        }
    }

    // the dimension named in a control file
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "overworld" => Some(Self::Overworld),
            "nether" => Some(Self::Nether),
            "end" => Some(Self::End),
            _ => None,
        }
    }
}
