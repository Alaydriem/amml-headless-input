//! The headless-input MCP server and hub. The first instance to bind the hub port serves MCP for every instance;
//! the others register with it as members and take over when it exits.

pub mod claude_cli;
pub mod dto;
pub mod errors;
pub mod hub;
pub mod pair_client;
pub mod port;
pub mod server;
pub mod token;

pub use hub::{HubNode, HubRole, InstanceId};
pub use port::{HeadlessPort, PortError};
pub use token::{LegacyCopy, LegacyTokenFile, TokenStore};
