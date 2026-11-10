use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;

use crate::port::HeadlessPort;

// Where the hub reads a control file the tools name by path: an absolute path as given, any other path (a bare file
// name among them) inside the control folder of the hub's own instance. The folder is asked for on each read, so a
// changed setting applies at once.
#[derive(Clone)]
pub struct ControlFolder {
    folder: Arc<dyn Fn() -> PathBuf + Send + Sync>,
}

impl ControlFolder {
    pub fn new<P: HeadlessPort>(port: Arc<P>) -> Self {
        Self { folder: Arc::new(move || port.control_folder()) }
    }

    // the file `path` names. Blocks: the folder comes from the port
    pub fn resolve(&self, path: &str) -> PathBuf {
        let path = PathBuf::from(path);
        if path.is_absolute() { path } else { (self.folder)().join(path) }
    }
}

impl fmt::Debug for ControlFolder {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ControlFolder")
    }
}
