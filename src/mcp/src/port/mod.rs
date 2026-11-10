pub mod host_resolver;
pub mod port_error;

pub use host_resolver::HostResolver;
pub use port_error::PortError;

use std::path::PathBuf;

use headless_input_core::control::ControlFile;
use headless_input_core::run::{LiveAction, RunDefaults, RunId, RunStatus};
use headless_input_core::structs::Capabilities;

// What the hub can ask of the local game instance. Every method blocks; the hub calls each one inside
// `tokio::task::spawn_blocking`.
pub trait HeadlessPort: Send + Sync + 'static {
    // starts a checked control file; the hub checked it with `features` and `defaults` of this instance
    fn run_control_file(&self, file: ControlFile) -> Result<RunId, PortError>;
    // ends the active run or live session
    fn stop(&self) -> Result<(), PortError>;
    fn status(&self) -> RunStatus;
    fn live(&self, action: LiveAction) -> Result<(), PortError>;
    // the reads and game calls the game build gives
    fn features(&self) -> Capabilities;
    // the settings a control file falls back on
    fn defaults(&self) -> RunDefaults;
    // the folder a control-file path that is not absolute, a bare file name among them, is read from
    fn control_folder(&self) -> PathBuf;
    // asks the person whether `client` may have a token; false on deny or no answer
    fn approve_pairing(&self, client: &str) -> bool;
}
