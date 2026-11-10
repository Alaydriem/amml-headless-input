use std::sync::Arc;

use crate::dto::{MemberReply, MemberRequest, StatusDto};
use crate::port::{HeadlessPort, HostResolver, PortError};

// Runs one forwarded command on the local instance. A control file is loaded with the defaults and features of this
// instance.
pub struct MemberExecutor;

impl MemberExecutor {
    pub fn execute<P: HeadlessPort>(port: &P, resolver: &HostResolver, request: MemberRequest) -> MemberReply {
        let result = match request {
            MemberRequest::RunControlFile(text) => resolver
                .load_control(&text, &port.defaults(), &port.features())
                .and_then(|file| port.run_control_file(file))
                .map(|run| MemberReply::Run(run.0)),
            MemberRequest::Stop => port.stop().map(|()| MemberReply::Done),
            MemberRequest::Status => Ok(MemberReply::Status(Box::new(StatusDto::from(&port.status())))),
            MemberRequest::Live(action) => {
                resolver.live(&action).and_then(|action| port.live(action)).map(|()| MemberReply::Done)
            }
        };
        result.unwrap_or_else(MemberReply::Failed)
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
