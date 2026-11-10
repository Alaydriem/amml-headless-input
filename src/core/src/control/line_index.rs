// The lines of the blocks and attributes of a control file text, to name the line of a refusal.
pub struct LineIndex {
    // ident, first label and 1-based line of each block start, in text order
    blocks: Vec<(String, Option<String>, usize)>,
    // key and 1-based line of each attribute line
    attributes: Vec<(String, usize)>,
}

impl LineIndex {
    pub fn new(text: &str) -> Self {
        let mut blocks = Vec::new();
        let mut attributes = Vec::new();
        for (index, line) in text.lines().enumerate() {
            let line_no = index + 1;
            let trimmed = line.trim_start();
            let ident: String =
                trimmed.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-').collect();
            if ident.is_empty() {
                continue;
            }
            let rest = trimmed[ident.len()..].trim_start();
            if rest.starts_with('=') {
                attributes.push((ident, line_no));
            } else if rest.starts_with('{') {
                blocks.push((ident, None, line_no));
            } else if let Some(label) = rest.strip_prefix('"')
                && let Some(end) = label.find('"')
                && label[end + 1..].contains('{')
            {
                blocks.push((ident, Some(label[..end].to_string()), line_no));
            }
        }
        Self { blocks, attributes }
    }

    // the line of the first block with this ident and label at or after the cursor; the cursor then moves past it
    pub fn block_line(&self, ident: &str, label: Option<&str>, cursor: &mut usize) -> Option<usize> {
        let start = (*cursor).min(self.blocks.len());
        let found = self.blocks[start..].iter().position(|(i, l, _)| i == ident && l.as_deref() == label)?;
        let at = start + found;
        *cursor = at + 1;
        Some(self.blocks[at].2)
    }

    // the line of the first block with this ident at or after the cursor, whatever its labels; the cursor then moves
    // past it
    pub fn ident_line(&self, ident: &str, cursor: &mut usize) -> Option<usize> {
        let start = (*cursor).min(self.blocks.len());
        let found = self.blocks[start..].iter().position(|(i, _, _)| i == ident)?;
        let at = start + found;
        *cursor = at + 1;
        Some(self.blocks[at].2)
    }

    // the line of the first attribute line with this key
    pub fn attribute_line(&self, key: &str) -> Option<usize> {
        self.attributes.iter().find(|(k, _)| k == key).map(|(_, line)| *line)
    }
}
