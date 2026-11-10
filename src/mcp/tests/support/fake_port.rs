use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::Receiver;

use headless_input_core::control::ControlFile;
use headless_input_core::run::{LiveAction, RunDefaults, RunId, RunState, RunStatus};
use headless_input_core::structs::Capabilities;
use headless_input_mcp::port::{HeadlessPort, PortError};

// How a `FakePort` answers.
pub struct FakeSetup {
    pub features: Capabilities,
    // what `approve_pairing` answers
    pub pairing: bool,
    // `approve_pairing` waits for one message here before it answers
    pub pairing_gate: Option<Receiver<()>>,
    // what `run_control_file` fails with
    pub run_error: Option<PortError>,
    // what `control_folder` answers
    pub folder: PathBuf,
}

impl Default for FakeSetup {
    fn default() -> Self {
        Self {
            features: Capabilities::all(),
            pairing: false,
            pairing_gate: None,
            run_error: None,
            folder: std::env::temp_dir(),
        }
    }
}

// A port that records each call and returns canned values.
pub struct FakePort {
    calls: Mutex<Vec<String>>,
    features: Capabilities,
    pairing: bool,
    pairing_gate: Option<Mutex<Receiver<()>>>,
    run_error: Option<PortError>,
    folder: PathBuf,
    next_run: AtomicU64,
}

impl FakePort {
    pub fn new(setup: FakeSetup) -> Self {
        Self {
            calls: Mutex::new(Vec::new()),
            features: setup.features,
            pairing: setup.pairing,
            pairing_gate: setup.pairing_gate.map(Mutex::new),
            run_error: setup.run_error,
            folder: setup.folder,
            next_run: AtomicU64::new(1),
        }
    }

    pub fn plain() -> Self {
        Self::new(FakeSetup::default())
    }

    pub fn pairing(answer: bool) -> Self {
        Self::new(FakeSetup { pairing: answer, ..FakeSetup::default() })
    }

    pub fn calls(&self) -> Vec<String> {
        self.calls.lock().unwrap().clone()
    }

    fn record(&self, call: String) {
        self.calls.lock().unwrap().push(call);
    }
}

impl HeadlessPort for FakePort {
    fn run_control_file(&self, file: ControlFile) -> Result<RunId, PortError> {
        self.record(format!("run_control_file {}", file.name));
        if let Some(error) = &self.run_error {
            return Err(error.clone());
        }
        Ok(RunId(self.next_run.fetch_add(1, Ordering::SeqCst)))
    }

    fn stop(&self) -> Result<(), PortError> {
        self.record("stop".to_owned());
        Ok(())
    }

    fn status(&self) -> RunStatus {
        self.record("status".to_owned());
        RunStatus {
            state: RunState::Idle,
            run: None,
            name: None,
            cursor: None,
            ticks: 0,
            last: None,
            position: [1.0, 64.0, 2.0],
            yaw: 90.0,
            pitch: 0.0,
            dimension: Some(0),
        }
    }

    fn live(&self, action: LiveAction) -> Result<(), PortError> {
        self.record(format!("live {action:?}"));
        Ok(())
    }

    fn features(&self) -> Capabilities {
        self.features
    }

    fn defaults(&self) -> RunDefaults {
        RunDefaults::default()
    }

    fn control_folder(&self) -> PathBuf {
        self.folder.clone()
    }

    fn approve_pairing(&self, client: &str) -> bool {
        self.record(format!("approve_pairing {client}"));
        if let Some(gate) = &self.pairing_gate {
            let _ = gate.lock().unwrap().recv();
        }
        self.pairing
    }
}
