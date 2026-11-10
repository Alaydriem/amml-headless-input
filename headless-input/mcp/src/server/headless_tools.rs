use std::sync::Arc;

use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, Implementation, ServerCapabilities, ServerInfo};
use rmcp::{ErrorData, ServerHandler, tool, tool_handler, tool_router};

use super::ToolReply;
use crate::dto::{
    HotbarArgs, InstanceArgs, JoinServerArgs, KeyArgs, LeaveWorldArgs, LiveActionDto, MemberRequest, MoveArgs,
    PointerArgs, RunControlFileArgs, ScrollArgs, TurnArgs,
};
use crate::hub::{ControlFolder, Forwarder, HubCalls};
use crate::port::HostResolver;

// The rmcp tool router. Each tool resolves the instance it names and forwards one command through the hub.
#[derive(Clone)]
pub struct HeadlessTools {
    calls: HubCalls,
    tool_router: ToolRouter<Self>,
}

impl HeadlessTools {
    pub const SERVER: &'static str = "headless-input";
    pub const INSTRUCTIONS: &'static str = "Drives the local player in one or more running Minecraft Bedrock clients.";

    pub fn new(forwarder: Arc<Forwarder>, folder: ControlFolder, resolver: Arc<HostResolver>) -> Self {
        Self { calls: HubCalls::new(forwarder, folder, resolver), tool_router: Self::tool_router() }
    }

    async fn live(&self, instance: Option<String>, action: LiveActionDto) -> Result<CallToolResult, ErrorData> {
        ToolReply::port(self.calls.live(instance.as_deref(), action).await)
    }
}

#[tool_router]
impl HeadlessTools {
    #[tool(
        description = "The connected instances: id, whether it is the hub, the process id, the state (idle, running, live) and the features its game build gives."
    )]
    async fn instances(&self) -> Result<CallToolResult, ErrorData> {
        ToolReply::json(&self.calls.instances().await)
    }

    #[tool(
        description = "Run an HCL control file, given as a path or as text. The hub checks it, then starts it on the instance (\"*\" for every instance, with one result each). Returns at once with the run id; poll status for the end."
    )]
    async fn run_control_file(
        &self,
        Parameters(args): Parameters<RunControlFileArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        ToolReply::port(self.calls.run_control_file(args.instance.as_deref(), args.path, args.text).await)
    }

    #[tool(description = "End the active run or live session (\"*\" for every instance).")]
    async fn stop(&self, Parameters(args): Parameters<InstanceArgs>) -> Result<CallToolResult, ErrorData> {
        ToolReply::port(self.calls.each(args.instance.as_deref(), MemberRequest::Stop).await)
    }

    #[tool(
        description = "Whether a run or live session is active, the current step and repeat passes, ticks elapsed, the last result, position, yaw, pitch and dimension."
    )]
    async fn status(&self, Parameters(args): Parameters<InstanceArgs>) -> Result<CallToolResult, ErrorData> {
        ToolReply::port(self.calls.one(args.instance.as_deref(), MemberRequest::Status).await)
    }

    #[tool(
        name = "move",
        description = "Hold movement keys (forward, back, left, right) for a duration, or until the live session ends."
    )]
    async fn move_keys(&self, Parameters(args): Parameters<MoveArgs>) -> Result<CallToolResult, ErrorData> {
        self.live(args.instance, LiveActionDto::Hold { keys: args.hold, for_time: args.for_time }).await
    }

    #[tool(description = "Hold jump for a duration, or until the live session ends.")]
    async fn jump(&self, Parameters(args): Parameters<KeyArgs>) -> Result<CallToolResult, ErrorData> {
        self.live(args.instance, LiveActionDto::Hold { keys: vec!["jump".to_owned()], for_time: args.for_time }).await
    }

    #[tool(description = "Hold sneak for a duration, or until the live session ends.")]
    async fn sneak(&self, Parameters(args): Parameters<KeyArgs>) -> Result<CallToolResult, ErrorData> {
        self.live(args.instance, LiveActionDto::Hold { keys: vec!["sneak".to_owned()], for_time: args.for_time }).await
    }

    #[tool(description = "Hold sprint for a duration, or until the live session ends.")]
    async fn sprint(&self, Parameters(args): Parameters<KeyArgs>) -> Result<CallToolResult, ErrorData> {
        self.live(args.instance, LiveActionDto::Hold { keys: vec!["sprint".to_owned()], for_time: args.for_time }).await
    }

    #[tool(description = "Turn by yaw and pitch degrees over a duration.")]
    async fn turn(&self, Parameters(args): Parameters<TurnArgs>) -> Result<CallToolResult, ErrorData> {
        self.live(args.instance, LiveActionDto::Turn { yaw: args.yaw, pitch: args.pitch, over: args.over }).await
    }

    #[tool(description = "Turn to face yaw and pitch degrees over a duration.")]
    async fn look_at(&self, Parameters(args): Parameters<TurnArgs>) -> Result<CallToolResult, ErrorData> {
        self.live(args.instance, LiveActionDto::LookAt { yaw: args.yaw, pitch: args.pitch, over: args.over }).await
    }

    #[tool(description = "Attack: one click, or hold for a duration.")]
    async fn attack(&self, Parameters(args): Parameters<PointerArgs>) -> Result<CallToolResult, ErrorData> {
        self.live(args.instance, LiveActionDto::Pointer { action: "attack".to_owned(), hold: args.hold }).await
    }

    #[tool(name = "use", description = "Use: one click, or hold for a duration.")]
    async fn use_item(&self, Parameters(args): Parameters<PointerArgs>) -> Result<CallToolResult, ErrorData> {
        self.live(args.instance, LiveActionDto::Pointer { action: "use".to_owned(), hold: args.hold }).await
    }

    #[tool(description = "Select a hotbar slot, 1 to 9.")]
    async fn hotbar(&self, Parameters(args): Parameters<HotbarArgs>) -> Result<CallToolResult, ErrorData> {
        self.live(args.instance, LiveActionDto::Hotbar { slot: args.slot }).await
    }

    #[tool(description = "Scroll the mouse wheel by steps; positive scrolls up.")]
    async fn scroll(&self, Parameters(args): Parameters<ScrollArgs>) -> Result<CallToolResult, ErrorData> {
        self.live(args.instance, LiveActionDto::Scroll { steps: args.steps }).await
    }

    #[tool(description = "Join a server by address and port (default 19132); leaves the current world first.")]
    async fn join_server(&self, Parameters(args): Parameters<JoinServerArgs>) -> Result<CallToolResult, ErrorData> {
        let action = LiveActionDto::JoinServer {
            address: args.address,
            port: args.port.unwrap_or(JoinServerArgs::DEFAULT_PORT),
            timeout: args.timeout,
        };
        self.live(args.instance, action).await
    }

    #[tool(description = "Leave the current world.")]
    async fn leave_world(&self, Parameters(args): Parameters<LeaveWorldArgs>) -> Result<CallToolResult, ErrorData> {
        self.live(args.instance, LiveActionDto::LeaveWorld { timeout: args.timeout }).await
    }

    #[tool(description = "End the live session and release every held input (\"*\" for every instance).")]
    async fn release(&self, Parameters(args): Parameters<InstanceArgs>) -> Result<CallToolResult, ErrorData> {
        ToolReply::port(self.calls.each(args.instance.as_deref(), MemberRequest::Live(LiveActionDto::Release)).await)
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for HeadlessTools {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new(Self::SERVER, env!("CARGO_PKG_VERSION")))
            .with_instructions(Self::INSTRUCTIONS)
    }
}
