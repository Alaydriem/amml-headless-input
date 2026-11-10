//! Host resolution: literals, local names and names that never resolve.

use std::net::Ipv4Addr;

use headless_input_mcp::PortError;
use headless_input_mcp::port::HostResolver;

#[test]
fn an_ipv4_literal_resolves_to_itself() {
    assert_eq!(HostResolver::new().resolve("10.1.2.3", 19132), Ok(Ipv4Addr::new(10, 1, 2, 3)));
}

#[test]
fn localhost_resolves_to_loopback() {
    assert_eq!(HostResolver::new().resolve("localhost", 19132), Ok(Ipv4Addr::LOCALHOST));
}

#[test]
fn an_unresolvable_host_is_an_error_naming_it() {
    let error = HostResolver::new().resolve("name.invalid", 19132).unwrap_err();
    assert_eq!(error, PortError::Unresolved { host: "name.invalid".to_owned() });
    assert_eq!(error.to_string(), "could not resolve name.invalid");
}

#[test]
fn a_lookup_slower_than_the_timeout_is_unresolved() {
    let started = std::time::Instant::now();
    let result = HostResolver::new().bounded("slow.example", std::time::Duration::from_millis(50), || {
        std::thread::sleep(std::time::Duration::from_millis(1500));
        Ok(Ipv4Addr::new(1, 2, 3, 4))
    });
    assert_eq!(result, Err(PortError::Unresolved { host: "slow.example".to_owned() }));
    assert!(started.elapsed() < std::time::Duration::from_millis(1000), "{:?}", started.elapsed());
}

#[test]
fn a_lookup_inside_the_timeout_gives_its_answer() {
    let result = HostResolver::new()
        .bounded("fast.example", std::time::Duration::from_secs(5), || Ok(Ipv4Addr::new(1, 2, 3, 4)));
    assert_eq!(result, Ok(Ipv4Addr::new(1, 2, 3, 4)));
}

#[test]
fn the_ninth_concurrent_lookup_is_unresolved_at_once() {
    let resolver = HostResolver::new();
    let slow = || {
        std::thread::sleep(std::time::Duration::from_millis(1500));
        Ok(Ipv4Addr::new(1, 2, 3, 4))
    };
    for _ in 0..HostResolver::MAX_LOOKUPS {
        let held = resolver.bounded("held.example", std::time::Duration::from_millis(20), slow);
        assert_eq!(held, Err(PortError::Unresolved { host: "held.example".to_owned() }));
    }

    let started = std::time::Instant::now();
    let ninth = resolver.bounded("ninth.example", std::time::Duration::from_secs(5), || Ok(Ipv4Addr::new(5, 6, 7, 8)));
    assert_eq!(ninth, Err(PortError::Unresolved { host: "ninth.example".to_owned() }));
    assert!(started.elapsed() < std::time::Duration::from_millis(500), "{:?}", started.elapsed());

    let other = HostResolver::new();
    let free = other.bounded("other.example", std::time::Duration::from_secs(5), || Ok(Ipv4Addr::new(9, 9, 9, 9)));
    assert_eq!(free, Ok(Ipv4Addr::new(9, 9, 9, 9)));
}
