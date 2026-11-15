use std::sync::Arc;

use super::WorldLookup;
use headless_input_core::structs::ScreenshotRequest;

use crate::dto::{MemberReply, MemberRequest, ScreenshotDto, StatusDto, WorldDto};
use crate::port::{HeadlessPort, HostResolver, PortError};

// Runs one forwarded command on the local instance. A control file is loaded with the defaults and features of this
// instance.
pub struct MemberExecutor;

impl MemberExecutor {
    pub fn execute<P: HeadlessPort>(port: &P, resolver: &HostResolver, request: MemberRequest) -> MemberReply {
        let worlds = WorldLookup::new(port);
        let result = match request {
            MemberRequest::RunControlFile(text) => resolver
                .load_control(&text, &port.defaults(), &port.features(), |world| worlds.find(world))
                .map_err(|error| Self::world_failure(&worlds, error))
                .and_then(|file| port.run_control_file(file))
                .map(|run| MemberReply::Run(run.0)),
            MemberRequest::Stop => port.stop().map(|()| MemberReply::Done),
            MemberRequest::Status => Ok(MemberReply::Status(Box::new(StatusDto::from(&port.status())))),
            MemberRequest::Live(action) => resolver
                .live(&action, |world| worlds.find(world))
                .map_err(|error| Self::world_failure(&worlds, error))
                .and_then(|action| port.live(action))
                .map(|()| MemberReply::Done),
            MemberRequest::ListWorlds => {
                port.worlds().map(|catalog| MemberReply::Worlds(catalog.entries().iter().map(WorldDto::from).collect()))
            }
            MemberRequest::Screenshot { layers, max_width } => ScreenshotRequest::parse(layers.as_deref(), max_width)
                .map_err(PortError::Bad)
                .and_then(|request| port.screenshot(request))
                .map(|shot| MemberReply::Screenshot(Box::new(ScreenshotDto::from(&shot)))),
        };
        result.unwrap_or_else(MemberReply::Failed)
    }

    // a world that was not found because the catalog could not be read fails with that read error
    fn world_failure<P: HeadlessPort>(worlds: &WorldLookup<'_, P>, error: PortError) -> PortError {
        match (&error, worlds.failure()) {
            (PortError::UnknownWorld { .. } | PortError::Bad(_), Some(read)) => read,
            _ => error,
        }
    }

    // `execute` on a blocking thread; a panic in the port is an `Io` failure
    pub async fn execute_blocking<P: HeadlessPort>(
        port: Arc<P>,
        resolver: Arc<HostResolver>,
        request: MemberRequest,
    ) -> MemberReply {
        tokio::task::spawn_blocking(move || Self::execute(port.as_ref(), &resolver, request))
            .await
            .unwrap_or_else(|_| MemberReply::Failed(PortError::Io("the instance failed on the command".to_owned())))
    }
}
