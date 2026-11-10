//! Runs the Claude Code CLI to add the headless-input MCP server: `claude mcp remove` (a missing entry is fine), then
//! `claude mcp add --transport http headless-input http://127.0.0.1:<port>/mcp --header "Authorization: Bearer <token>"`.

mod claude_outcome;

pub use claude_outcome::ClaudeOutcome;

use std::io::{self, Read};
use std::process::{Child, Command, ExitStatus, Output, Stdio};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// The `claude` command lines for the headless-input server, and a runner for them. `scope` is the `--scope` value
/// (`SCOPE` for both paths). Each `claude` run ends within `timeout`; a child that takes longer is killed.
pub struct ClaudeCli {
    programs: Vec<String>,
    timeout: Duration,
}

impl ClaudeCli {
    /// The name of the server entry in Claude Code.
    pub const SERVER: &'static str = "headless-input";
    /// The scope `mise run mcp:connect` uses, so Claude Code holds one entry for every project.
    pub const SCOPE: &'static str = "user";
    /// Windows flag: start the child with no console window.
    #[cfg(windows)]
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    /// The longest one `claude` run may take.
    pub const TIMEOUT: Duration = Duration::from_secs(30);
    /// How often the runner looks whether the child has ended.
    const POLL: Duration = Duration::from_millis(20);

    /// A runner that tries `programs` in order, each run ending within `timeout`.
    pub fn new(programs: Vec<String>, timeout: Duration) -> Self {
        Self { programs, timeout }
    }

    /// The runner for `claude` (and on Windows the npm `claude.cmd` shim), with `TIMEOUT`.
    pub fn system() -> Self {
        let programs = if cfg!(windows) { vec!["claude", "claude.cmd"] } else { vec!["claude"] };
        Self::new(programs.into_iter().map(str::to_owned).collect(), Self::TIMEOUT)
    }

    pub fn url(port: u16) -> String {
        format!("http://127.0.0.1:{port}/mcp")
    }

    /// The arguments of `claude mcp add`.
    pub fn add_args(port: u16, token: &str, scope: Option<&str>) -> Vec<String> {
        let mut args = vec!["mcp".to_owned(), "add".to_owned()];
        if let Some(scope) = scope {
            args.extend(["--scope".to_owned(), scope.to_owned()]);
        }
        args.extend([
            "--transport".to_owned(),
            "http".to_owned(),
            Self::SERVER.to_owned(),
            Self::url(port),
            "--header".to_owned(),
            format!("Authorization: Bearer {token}"),
        ]);
        args
    }

    /// The arguments of `claude mcp remove`.
    pub fn remove_args(scope: Option<&str>) -> Vec<String> {
        let mut args = vec!["mcp".to_owned(), "remove".to_owned()];
        if let Some(scope) = scope {
            args.extend(["--scope".to_owned(), scope.to_owned()]);
        }
        args.push(Self::SERVER.to_owned());
        args
    }

    /// `claude mcp add ...` as one line to type, with the header in quotes.
    pub fn add_line(port: u16, token: &str, scope: Option<&str>) -> String {
        let scope = scope.map(|scope| format!("--scope {scope} ")).unwrap_or_default();
        format!(
            "claude mcp add {scope}--transport http {} {} --header \"Authorization: Bearer {token}\"",
            Self::SERVER,
            Self::url(port)
        )
    }

    /// Removes the old entry (a failure is ignored) and adds the new one. Blocks until `claude` ends, at most
    /// `timeout` for each run.
    pub fn register(&self, port: u16, token: &str, scope: Option<&str>) -> ClaudeOutcome {
        let Some(program) = self.find() else {
            return ClaudeOutcome::NotFound(Self::add_line(port, token, scope));
        };
        let _ = self.run(&program, &Self::remove_args(scope));
        match self.run(&program, &Self::add_args(port, token, scope)) {
            Ok(output) if output.status.success() => ClaudeOutcome::Added,
            Ok(output) => {
                let mut text = String::from_utf8_lossy(&output.stderr).trim().to_owned();
                if text.is_empty() {
                    text = String::from_utf8_lossy(&output.stdout).trim().to_owned();
                }
                ClaudeOutcome::Failed(text)
            }
            Err(error) => ClaudeOutcome::Failed(error.to_string()),
        }
    }

    /// Runs `program` with `args` and waits at most `timeout`; a child that takes longer is killed and the result
    /// is a `TimedOut` error.
    pub fn run(&self, program: &str, args: &[String]) -> io::Result<Output> {
        let mut command = Command::new(program);
        command.args(args).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(Self::CREATE_NO_WINDOW);
        }
        let mut child = command.spawn()?;
        // the pipes are read on their own threads, so a child that writes much never waits on a full pipe
        let stdout = Self::drain(child.stdout.take());
        let stderr = Self::drain(child.stderr.take());
        // after a time-out the pipe threads are left to end on their own: a grandchild may still hold the pipes
        let status = self.wait(&mut child);
        let output = |status: ExitStatus| Output {
            status,
            stdout: stdout.join().unwrap_or_default(),
            stderr: stderr.join().unwrap_or_default(),
        };
        status.map(output)
    }

    // the first program name that starts
    fn find(&self) -> Option<String> {
        self.programs
            .iter()
            .find(|name| match self.run(name, &["--version".to_owned()]) {
                Ok(_) => true,
                Err(error) => error.kind() != io::ErrorKind::NotFound,
            })
            .cloned()
    }

    fn wait(&self, child: &mut Child) -> io::Result<ExitStatus> {
        let start = Instant::now();
        loop {
            if let Some(status) = child.try_wait()? {
                return Ok(status);
            }
            if start.elapsed() >= self.timeout {
                Self::kill_tree(child);
                let _ = child.kill();
                let _ = child.wait();
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    format!("claude did not finish in {} s and was stopped", self.timeout.as_secs_f32()),
                ));
            }
            std::thread::sleep(Self::POLL);
        }
    }

    // `claude.cmd` starts node under cmd.exe: on Windows the whole tree goes, so no child is left running
    fn kill_tree(child: &Child) {
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            let _ = Command::new("taskkill")
                .args(["/PID", &child.id().to_string(), "/T", "/F"])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .creation_flags(Self::CREATE_NO_WINDOW)
                .status();
        }
        #[cfg(not(windows))]
        let _ = child;
    }

    fn drain(pipe: Option<impl Read + Send + 'static>) -> JoinHandle<Vec<u8>> {
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            if let Some(mut pipe) = pipe {
                let _ = pipe.read_to_end(&mut bytes);
            }
            bytes
        })
    }
}
