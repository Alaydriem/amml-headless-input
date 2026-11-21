//! @alaydriem/headless-input as an AMML mod: listens to the client frame and the game tick, reads the local player
//! and the manager events through the loader, and drives headless-input-core with them.

#![deny(unsafe_code)]

pub mod command;
pub mod errors;
pub mod game;
pub mod headless_mod;
pub mod join;
pub mod output;
pub mod services;
pub mod traits;
pub mod types;

pub use headless_mod::HeadlessMod;

amml_api::amml_mod!(HeadlessMod);
