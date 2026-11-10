use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use alaydriem_headless_input::services::{HeadlessRuntime, LocalPort};
use alaydriem_headless_input::types::FrameView;
use headless_input_core::control::{ControlFile, ControlLoader};
use headless_input_core::player::{PlayerView, ScreenKind};
use headless_input_core::run::{RunDefaults, RunId};
use headless_input_core::structs::Capabilities;
use headless_input_mcp::port::{HeadlessPort, PortError};

use super::fake_command_client::FakeCommandClient;
use super::fake_join_client::FakeJoinClient;
use super::fake_manager::FakeManager;
use super::runtime_dirs::RuntimeDirs;

// a runtime driven by hand: the test gives it frames and ticks, with a fake manager and a fake game client, and asks
// it through its local port from other threads
pub struct RunHarness {
    pub runtime: HeadlessRuntime,
    pub manager: FakeManager,
    pub join: FakeJoinClient,
    pub command: FakeCommandClient,
}

impl RunHarness {
    pub const WAIT: Duration = Duration::from_secs(5);
    // a run of 100 ticks that holds forward
    pub const WALK: &'static str = "name = \"walk\"\n\nstep \"move\" {\n  hold = [\"forward\"]\n  for  = \"5s\"\n}\n";

    pub fn new() -> Self {
        Self::with_manager(FakeManager::new())
    }

    pub fn with_manager(manager: FakeManager) -> Self {
        Self {
            runtime: HeadlessRuntime::new(&RuntimeDirs::dirs()),
            manager,
            join: FakeJoinClient::in_world(),
            command: FakeCommandClient::new(),
        }
    }

    pub fn file(text: &str) -> ControlFile {
        ControlLoader::load(text, &RunDefaults::default(), &Capabilities::all(), |_| {
            Some(std::net::Ipv4Addr::LOCALHOST)
        })
        .unwrap()
    }

    pub fn world_view() -> PlayerView {
        PlayerView { position: [0.0, 64.0, 0.0], yaw: 0.0, pitch: 0.0, dimension: Some(0), screen: ScreenKind::InWorld }
    }

    pub fn world() -> FrameView {
        FrameView { view: Self::world_view(), in_world: true, fresh: true }
    }

    pub fn title() -> FrameView {
        let view = PlayerView { position: [0.0; 3], yaw: 0.0, pitch: 0.0, dimension: None, screen: ScreenKind::Title };
        FrameView { view, in_world: false, fresh: false }
    }

    pub fn port(&self) -> Arc<LocalPort> {
        self.runtime.local_port(Self::WAIT)
    }

    pub fn frame(&self, frame: &FrameView) {
        self.runtime.drive_frame(frame, Some(&self.manager), &self.join, &self.command);
    }

    pub fn tick(&self) {
        self.runtime.drive_tick(Some(&Self::world_view()), Some(&self.manager));
    }

    // a game tick of a client that has no local player
    pub fn tick_without_player(&self) {
        self.runtime.drive_tick(None, Some(&self.manager));
    }

    // asks the local port for `file` on another thread, and returns once the command waits in the queue
    pub fn queue_run(&self, file: ControlFile) -> JoinHandle<Result<RunId, PortError>> {
        let port = self.port();
        let asking = std::thread::spawn(move || port.run_control_file(file));
        self.wait_queued(1);
        asking
    }

    // starts `file` with one frame in the world
    pub fn start(&self, file: ControlFile) -> Result<RunId, PortError> {
        let asking = self.queue_run(file);
        self.frame(&Self::world());
        asking.join().unwrap()
    }

    pub fn wait_queued(&self, count: usize) {
        let deadline = Instant::now() + Self::WAIT;
        while self.runtime.queued() < count {
            assert!(Instant::now() < deadline, "the command did not reach the queue");
            std::thread::sleep(Duration::from_millis(1));
        }
    }
}
