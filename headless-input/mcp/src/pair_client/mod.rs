//! The client side of pairing, for the `headless_pair` binary (`mise run mcp:connect`).

mod pair_outcome;
mod raw_config;
mod raw_mcp;

pub use pair_outcome::PairOutcome;
pub use raw_config::RawConfig;
pub use raw_mcp::RawMcp;

use std::io::{self, Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;

use crate::claude_cli::{ClaudeCli, ClaudeOutcome};
use crate::dto::{PairReply, PairRequest};

// Sends `POST /pair` to the hub on `127.0.0.1:<port>` and waits for the answer of the person, then adds the server to
// Claude Code.
pub struct PairClient {
    port: u16,
    client: String,
    timeout: Duration,
}

impl PairClient {
    // the client name the binary sends
    pub const CLIENT: &'static str = "claude-code";
    // the hub port when the config does not name one
    pub const DEFAULT_PORT: u16 = 28250;
    // longer than the hub waits for the person
    pub const TIMEOUT: Duration = Duration::from_secs(75);
    // the variable that names the mods folder of the instance
    pub const MODS_DIR_VAR: &'static str = "HEADLESS_MODS_DIR";

    pub fn new(port: u16, client: &str) -> Self {
        Self { port, client: client.to_owned(), timeout: Self::TIMEOUT }
    }

    // the manager config of the mod: `<mods_dir>/amml/@alaydriem/headless-input/config/config.hcl`
    pub fn config_file(mods_dir: &Path) -> PathBuf {
        mods_dir.join("amml").join("@alaydriem").join("headless-input").join("config").join("config.hcl")
    }

    // `mcp.port` of the config; the default when the text or the value is not good
    pub fn port_from_config(text: &str) -> u16 {
        hcl::from_str::<RawConfig>(text)
            .ok()
            .and_then(|config| config.mcp.port)
            .and_then(|port| u16::try_from(port).ok())
            .filter(|port| *port >= 1024)
            .unwrap_or(Self::DEFAULT_PORT)
    }

    // sends the request and blocks until the answer (at most `TIMEOUT`)
    pub fn request(&self) -> PairOutcome {
        match self.exchange() {
            Ok(raw) => Self::read_reply(&raw),
            Err(error) if error.kind() == io::ErrorKind::ConnectionRefused => PairOutcome::NotRunning,
            Err(error) => PairOutcome::Failed(error.to_string()),
        }
    }

    // the outcome of one raw HTTP/1.1 response
    pub fn read_reply(raw: &[u8]) -> PairOutcome {
        let text = String::from_utf8_lossy(raw);
        let status = text.split_whitespace().nth(1).and_then(|code| code.parse::<u16>().ok());
        let body = text.split_once("\r\n\r\n").map_or("", |(_, body)| body);
        match status {
            Some(200) => match serde_json::from_str::<PairReply>(body.trim()) {
                Ok(reply) => PairOutcome::Token(reply.token),
                Err(error) => PairOutcome::Failed(format!("bad reply: {error}")),
            },
            Some(403) => PairOutcome::Denied,
            Some(409) => PairOutcome::Busy,
            Some(code) => PairOutcome::Failed(format!("the hub answered {code}")),
            None => PairOutcome::Failed("the hub gave no HTTP answer".to_owned()),
        }
    }

    // the whole `headless_pair` run: read the port, pair, add the server to Claude Code
    pub fn main() -> ExitCode {
        let Some(mods_dir) = std::env::var_os(Self::MODS_DIR_VAR) else {
            eprintln!("Set HEADLESS_MODS_DIR to the instance mods folder (mise reads it from .env).");
            return ExitCode::FAILURE;
        };
        let config = Self::config_file(Path::new(&mods_dir));
        let port =
            std::fs::read_to_string(&config).map(|text| Self::port_from_config(&text)).unwrap_or(Self::DEFAULT_PORT);
        println!("Approve the request in the game that is the headless-input hub (port {port}).");
        match Self::new(port, Self::CLIENT).request() {
            PairOutcome::Token(token) => Self::register(port, &token),
            PairOutcome::Denied => Self::fail("The hub denied the request (or no one answered in time)."),
            PairOutcome::Busy => {
                Self::fail("Another pairing request is waiting in the hub game. Answer it, then try again.")
            }
            PairOutcome::NotRunning => Self::fail(&format!(
                "No hub on port {port}: turn on the MCP server in the headless-input settings of a running game."
            )),
            PairOutcome::Failed(reason) => Self::fail(&format!("Pairing failed: {reason}")),
        }
    }

    fn register(port: u16, token: &str) -> ExitCode {
        match ClaudeCli::system().register(port, token, Some(ClaudeCli::SCOPE)) {
            ClaudeOutcome::Added => {
                println!("headless-input is added to Claude Code. Restart Claude Code to use it.");
                ExitCode::SUCCESS
            }
            ClaudeOutcome::NotFound(line) => {
                println!("The claude command is not on the path. Run this command:");
                println!("{line}");
                ExitCode::SUCCESS
            }
            ClaudeOutcome::Failed(reason) => {
                eprintln!("claude mcp add failed: {reason}");
                println!("Run this command by hand:");
                println!("{}", ClaudeCli::add_line(port, token, Some(ClaudeCli::SCOPE)));
                ExitCode::FAILURE
            }
        }
    }

    fn fail(message: &str) -> ExitCode {
        eprintln!("{message}");
        ExitCode::FAILURE
    }

    fn exchange(&self) -> io::Result<Vec<u8>> {
        let address = SocketAddr::from((Ipv4Addr::LOCALHOST, self.port));
        let mut stream = TcpStream::connect_timeout(&address, Duration::from_secs(5))?;
        stream.set_read_timeout(Some(self.timeout))?;
        stream.set_write_timeout(Some(self.timeout))?;
        let body = serde_json::to_string(&PairRequest { client: self.client.clone() }).map_err(io::Error::other)?;
        let request = format!(
            "POST /pair HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            self.port,
            body.len()
        );
        stream.write_all(request.as_bytes())?;
        let mut raw = Vec::new();
        stream.read_to_end(&mut raw)?;
        Ok(raw)
    }
}
