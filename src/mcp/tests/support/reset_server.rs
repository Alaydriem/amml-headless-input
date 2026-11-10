use std::net::{Ipv4Addr, TcpListener};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;

use socket2::SockRef;

// A program on a port that accepts each connection and resets it at once, with no HTTP answer. After `limit`
// connections it closes the port.
pub struct ResetServer {
    accepted: Arc<AtomicUsize>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl ResetServer {
    pub fn start(port: u16, limit: usize) -> Self {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, port)).expect("the reset server binds");
        listener.set_nonblocking(true).unwrap();
        let accepted = Arc::new(AtomicUsize::new(0));
        let stop = Arc::new(AtomicBool::new(false));
        let (count, stopping) = (Arc::clone(&accepted), Arc::clone(&stop));
        let thread = std::thread::spawn(move || {
            while !stopping.load(Ordering::SeqCst) && count.load(Ordering::SeqCst) < limit {
                match listener.accept() {
                    Ok((stream, _)) => {
                        let _ = SockRef::from(&stream).set_linger(Some(Duration::ZERO));
                        drop(stream);
                        count.fetch_add(1, Ordering::SeqCst);
                    }
                    Err(_) => std::thread::sleep(Duration::from_millis(5)),
                }
            }
        });
        Self { accepted, stop, thread: Some(thread) }
    }

    pub fn accepted(&self) -> usize {
        self.accepted.load(Ordering::SeqCst)
    }

    // waits until the port is closed (the limit was reached, or now)
    pub fn stop(mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
