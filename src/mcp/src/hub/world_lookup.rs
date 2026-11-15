use std::cell::OnceCell;

use headless_input_core::structs::WorldCatalog;

use crate::port::{HeadlessPort, PortError};

// The local worlds of one instance for one request. The catalog is read from the port on the first lookup, never
// before, and at most once.
pub struct WorldLookup<'a, P: HeadlessPort> {
    port: &'a P,
    catalog: OnceCell<Result<WorldCatalog, PortError>>,
}

impl<'a, P: HeadlessPort> WorldLookup<'a, P> {
    pub fn new(port: &'a P) -> Self {
        Self { port, catalog: OnceCell::new() }
    }

    // the level id of the world `text` names; None when no world matches or the catalog could not be read
    pub fn find(&self, text: &str) -> Option<String> {
        let catalog = self.catalog.get_or_init(|| self.port.worlds()).as_ref().ok()?;
        catalog.find(text).map(|entry| entry.level_id.clone())
    }

    // why the catalog could not be read, if it was asked for and failed
    pub fn failure(&self) -> Option<PortError> {
        self.catalog.get().and_then(|read| read.as_ref().err().cloned())
    }
}
