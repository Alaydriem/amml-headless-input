use std::cell::RefCell;
use std::net::{Ipv4Addr, ToSocketAddrs};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, mpsc};
use std::time::Duration;

use headless_input_core::control::{ControlFile, ControlLoader};
use headless_input_core::run::{LiveAction, RunDefaults};
use headless_input_core::structs::Capabilities;

use super::PortError;
use crate::dto::LiveActionDto;

// Turns a join host into an IPv4 address through the system resolver. Every method blocks on DNS, so callers run
// them inside `spawn_blocking`, never on the game thread.
#[derive(Debug)]
pub struct HostResolver {
    // lookups whose thread has not returned yet; at most `MAX_LOOKUPS`
    running: Arc<AtomicUsize>,
}

impl Default for HostResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl HostResolver {
    pub fn new() -> Self {
        Self { running: Arc::new(AtomicUsize::new(0)) }
    }

    pub fn new_shared() -> Arc<Self> {
        Arc::new(Self::new())
    }

    // the port a lookup asks for when the caller has none; the answer does not depend on it
    const LOOKUP_PORT: u16 = 19132;

    // the longest a lookup may take before the host counts as unresolved
    pub const TIMEOUT: Duration = Duration::from_secs(5);
    // lookups that may run at once; a lookup the system never answers keeps its thread, so the count is capped
    pub const MAX_LOOKUPS: usize = 8;

    // an IPv4 literal is its own address; a name gives the first IPv4 address of the lookup, or `Unresolved` when the
    // lookup fails, takes over `TIMEOUT`, or `MAX_LOOKUPS` lookups are already waiting
    pub fn resolve(&self, host: &str, port: u16) -> Result<Ipv4Addr, PortError> {
        if let Ok(address) = host.parse::<Ipv4Addr>() {
            return Ok(address);
        }
        let name = host.to_owned();
        self.bounded(host, Self::TIMEOUT, move || Self::system_lookup(&name, port))
    }

    // runs `lookup` on a helper thread and waits at most `timeout` for its answer on a one-slot channel
    pub fn bounded(
        &self,
        host: &str,
        timeout: Duration,
        lookup: impl FnOnce() -> Result<Ipv4Addr, PortError> + Send + 'static,
    ) -> Result<Ipv4Addr, PortError> {
        let running = Arc::clone(&self.running);
        let unresolved = || PortError::Unresolved { host: host.to_owned() };
        if running.fetch_add(1, Ordering::SeqCst) >= Self::MAX_LOOKUPS {
            running.fetch_sub(1, Ordering::SeqCst);
            return Err(unresolved());
        }
        let done = Arc::clone(&running);
        let (send, receive) = mpsc::sync_channel(1);
        let spawned = std::thread::Builder::new().name("host-lookup".to_owned()).spawn(move || {
            let answer = lookup();
            done.fetch_sub(1, Ordering::SeqCst);
            let _ = send.send(answer);
        });
        if spawned.is_err() {
            running.fetch_sub(1, Ordering::SeqCst);
            return Err(unresolved());
        }
        receive.recv_timeout(timeout).unwrap_or_else(|_| Err(unresolved()))
    }

    fn system_lookup(host: &str, port: u16) -> Result<Ipv4Addr, PortError> {
        (host, port)
            .to_socket_addrs()
            .ok()
            .and_then(|mut found| {
                found.find_map(|socket| match socket.ip() {
                    std::net::IpAddr::V4(address) => Some(address),
                    std::net::IpAddr::V6(_) => None,
                })
            })
            .ok_or_else(|| PortError::Unresolved { host: host.to_owned() })
    }

    // loads a control file, resolving each join host; a host that does not resolve gives `Unresolved`
    pub fn load_control(
        &self,
        text: &str,
        defaults: &RunDefaults,
        capabilities: &Capabilities,
    ) -> Result<ControlFile, PortError> {
        let failed = RefCell::new(None);
        let loaded = ControlLoader::load(text, defaults, capabilities, |host| self.lookup(host, &failed));
        loaded.map_err(|error| match failed.into_inner() {
            Some(host) => PortError::Unresolved { host },
            None => PortError::Bad(error.to_string()),
        })
    }

    // the live action of a tool call, resolving the join host if it has one
    pub fn live(&self, action: &LiveActionDto) -> Result<LiveAction, PortError> {
        let failed = RefCell::new(None);
        let built = action.action(|host| self.lookup(host, &failed));
        match (built, failed.into_inner()) {
            (Err(_), Some(host)) => Err(PortError::Unresolved { host }),
            (built, _) => built,
        }
    }

    fn lookup(&self, host: &str, failed: &RefCell<Option<String>>) -> Option<Ipv4Addr> {
        match self.resolve(host, Self::LOOKUP_PORT) {
            Ok(address) => Some(address),
            Err(_) => {
                failed.borrow_mut().get_or_insert_with(|| host.to_owned());
                None
            }
        }
    }
}
