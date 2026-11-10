//! `mise run mcp:connect`: asks the headless-input hub for an MCP token and adds the server to Claude Code.

fn main() -> std::process::ExitCode {
    headless_input_mcp::pair_client::PairClient::main()
}
