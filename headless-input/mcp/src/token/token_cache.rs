use super::IssuedToken;

// The tokens of the file as last read or written.
#[derive(Debug, Clone, Default)]
pub struct TokenCache {
    pub tokens: Vec<IssuedToken>,
}

impl TokenCache {
    pub fn new(tokens: Vec<IssuedToken>) -> Self {
        Self { tokens }
    }
}
