use std::net::{Ipv4Addr, TcpListener};
use std::thread::JoinHandle;

use tokio::sync::oneshot;

// Another program on a port: a router served on its own runtime, so `stop` closes every connection at once, as when
// a process ends.
pub struct SideServer {
    stop: Option<oneshot::Sender<()>>,
    thread: Option<JoinHandle<()>>,
}

impl SideServer {
    pub fn start(port: u16, router: axum::Router) -> Self {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, port)).expect("the side server binds");
        listener.set_nonblocking(true).unwrap();
        let (stop, stopped) = oneshot::channel::<()>();
        let thread = std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
            runtime.block_on(async move {
                let listener = tokio::net::TcpListener::from_std(listener).unwrap();
                tokio::select! {
                    _ = axum::serve(listener, router) => {}
                    _ = stopped => {}
                }
            });
            runtime.shutdown_background();
        });
        Self { stop: Some(stop), thread: Some(thread) }
    }

    pub fn stop(mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
