use crate::control::ControlFile;

// Where the player is in its control file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StepCursor {
    // the position of the step in the file, counting every step and repeat block in reading order from 0
    pub step: usize,
    // the ticks the step has run
    pub ticks_in_step: u32,
    // the passes each open repeat has finished, outermost first
    passes: [u32; ControlFile::MAX_DEPTH],
    depth: usize,
}

impl StepCursor {
    pub fn new(step: usize, ticks_in_step: u32, open_passes: &[u32]) -> Self {
        let mut passes = [0; ControlFile::MAX_DEPTH];
        let depth = open_passes.len().min(ControlFile::MAX_DEPTH);
        passes[..depth].copy_from_slice(&open_passes[..depth]);
        Self { step, ticks_in_step, passes, depth }
    }

    pub fn repeat_passes(&self) -> &[u32] {
        &self.passes[..self.depth]
    }
}
