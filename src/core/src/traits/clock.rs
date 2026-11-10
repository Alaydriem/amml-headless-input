use std::time::Instant;

// The wall clock the run safety limits read. Input is never placed by it.
pub trait Clock {
    fn now(&self) -> Instant;
}
