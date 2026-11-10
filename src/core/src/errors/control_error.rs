// A control file that was refused, with the line when the text names one.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{}", Self::describe(*.line, .reason))]
pub struct ControlError {
    pub line: Option<usize>,
    pub reason: String,
}

impl ControlError {
    pub fn new(line: Option<usize>, reason: impl Into<String>) -> Self {
        Self { line, reason: reason.into() }
    }

    fn describe(line: Option<usize>, reason: &str) -> String {
        match line {
            Some(line) => format!("line {line}: {reason}"),
            None => reason.to_string(),
        }
    }
}
