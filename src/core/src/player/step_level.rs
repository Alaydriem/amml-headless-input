// One open step list of the player: the top list of the file, or the block of a repeat.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct StepLevel {
    pub index: usize,
    pub passes: u32,
    // the player tick the repeat opened on
    pub opened: u64,
}
