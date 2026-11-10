//! @alaydriem/headless-input as an AMML mod: hooks the client update and the movement tick, reads the local player
//! and the manager events, and drives headless-input-core with them.

#![deny(unsafe_code)]

pub mod command;
pub mod detours;
pub mod errors;
pub mod game;
pub mod headless_mod;
pub mod join;
pub mod output;
pub mod services;
pub mod traits;
pub mod types;

pub use headless_mod::HeadlessMod;

amml_sdk::amml_mod!(HeadlessMod);
