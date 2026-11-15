use super::WorldEntry;

// A bounded list of local worlds in search order.
#[derive(Clone, Debug, Default)]
pub struct WorldCatalog {
    entries: Vec<WorldEntry>,
}

impl WorldCatalog {
    pub const MAX_WORLDS: usize = 1024;
    pub const MAX_NAME_BYTES: usize = 256;

    // Sorts into search order and keeps the first `MAX_WORLDS`.
    pub fn new(mut entries: Vec<WorldEntry>) -> Self {
        entries.sort_by(|a, b| (a.storage, &a.owner, &a.level_id).cmp(&(b.storage, &b.owner, &b.level_id)));
        entries.truncate(Self::MAX_WORLDS);
        Self { entries }
    }

    // Blank text finds nothing. An exact level id wins. Then the first name match, ignoring case and end whitespace.
    pub fn find(&self, text: &str) -> Option<&WorldEntry> {
        if text.trim().is_empty() {
            return None;
        }
        if let Some(found) = self.entries.iter().find(|w| w.level_id == text) {
            return Some(found);
        }
        let wanted = text.trim().to_lowercase();
        self.entries.iter().find(|w| w.name.trim().to_lowercase() == wanted)
    }

    pub fn entries(&self) -> &[WorldEntry] {
        &self.entries
    }

    // The first line of a `levelname.txt` file, read up to `MAX_NAME_BYTES`.
    pub fn name_from_bytes(bytes: &[u8]) -> String {
        let end = bytes.len().min(Self::MAX_NAME_BYTES);
        let text = String::from_utf8_lossy(&bytes[..end]);
        let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
        let line = text.split('\n').next().unwrap_or("");
        line.strip_suffix('\r').unwrap_or(line).to_string()
    }
}
