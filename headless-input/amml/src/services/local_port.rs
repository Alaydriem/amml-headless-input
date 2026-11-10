use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError, RwLock};
use std::time::Duration;

use headless_input_core::config::HeadlessConfig;
use headless_input_core::control::ControlFile;
use headless_input_core::run::{Command, LiveAction, RunDefaults, RunId, RunStatus};
use headless_input_core::structs::Capabilities;
use headless_input_mcp::port::{HeadlessPort, PortError};

use crate::services::{CommandInbox, CommandReply, PairingPrompt};

// This instance as the hub sees it. A command goes into the inbox and the call waits up to `wait` for the game
// thread to take it on its next frame; a call no frame takes times out and its command never runs. `status` reads
// the snapshot the game thread writes each frame. A live action that needs a read or call the game build does not
// give is refused at once. Every method may run on any thread
pub struct LocalPort {
    inbox: Arc<CommandInbox>,
    status: Arc<Mutex<RunStatus>>,
    capabilities: Arc<RwLock<Capabilities>>,
    config: Arc<RwLock<HeadlessConfig>>,
    prompt: Arc<PairingPrompt>,
    data_dir: PathBuf,
    wait: Duration,
}

impl LocalPort {
    // shorter than the hub's forward timeout, so the hub gets this call's answer
    pub const WAIT: Duration = Duration::from_secs(5);
    // the control folder inside the mod data folder when files.folder is empty
    pub const DEFAULT_FOLDER: &'static str = "control";

    pub fn new(
        inbox: Arc<CommandInbox>,
        status: Arc<Mutex<RunStatus>>,
        capabilities: Arc<RwLock<Capabilities>>,
        config: Arc<RwLock<HeadlessConfig>>,
        prompt: Arc<PairingPrompt>,
        data_dir: PathBuf,
        wait: Duration,
    ) -> Self {
        Self { inbox, status, capabilities, config, prompt, data_dir, wait }
    }

    fn send(&self, command: Command) -> Result<RunId, PortError> {
        let reply = Arc::new(CommandReply::new());
        self.inbox.push(command, Arc::clone(&reply))?;
        reply.wait(self.wait)
    }
}

impl HeadlessPort for LocalPort {
    fn run_control_file(&self, file: ControlFile) -> Result<RunId, PortError> {
        self.send(Command::RunFile { file })
    }

    fn stop(&self) -> Result<(), PortError> {
        self.send(Command::Stop).map(|_| ())
    }

    fn status(&self) -> RunStatus {
        self.status.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }

    fn live(&self, action: LiveAction) -> Result<(), PortError> {
        if let Some(missing) = self.features().missing_for_live(&action) {
            return Err(PortError::Refused(format!("this game build does not give {missing}")));
        }
        self.send(Command::Live(action)).map(|_| ())
    }

    fn features(&self) -> Capabilities {
        *self.capabilities.read().unwrap_or_else(PoisonError::into_inner)
    }

    fn defaults(&self) -> RunDefaults {
        RunDefaults::new(&self.config.read().unwrap_or_else(PoisonError::into_inner))
    }

    fn control_folder(&self) -> PathBuf {
        let config = self.config.read().unwrap_or_else(PoisonError::into_inner);
        if config.files_folder.is_empty() {
            self.data_dir.join(Self::DEFAULT_FOLDER)
        } else {
            PathBuf::from(&config.files_folder)
        }
    }

    fn approve_pairing(&self, client: &str) -> bool {
        self.prompt.ask(client)
    }
}
