use std::sync::Arc;
use std::time::Duration;

use axum::Json;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use tokio_util::sync::CancellationToken;

use super::{Forwarder, InstanceId, MemberInfo, MemberRegistry};
use crate::dto::{HubHello, MemberRef, RegisterRequest, ReplyPost};
use crate::errors::RegistryError;

// The hub's member endpoint. `GET /member/hello` has no bearer check and names the hub; the others take the bearer
// token of the shared store. A poll blocks up to `POLL_WAIT` for the next command: `200` with the command, `204`
// with none, `404` for a member the hub does not know, `503` while the hub stops.
pub struct MemberRoute {
    forwarder: Arc<Forwarder>,
    cancel: CancellationToken,
}

impl MemberRoute {
    pub const HELLO: &'static str = "/member/hello";
    pub const REGISTER: &'static str = "/member/register";
    pub const POLL: &'static str = "/member/poll";
    pub const REPLY: &'static str = "/member/reply";
    pub const HEARTBEAT: &'static str = "/member/heartbeat";
    pub const UNREGISTER: &'static str = "/member/unregister";
    pub const POLL_WAIT: Duration = Duration::from_secs(25);

    pub fn new(forwarder: Arc<Forwarder>, cancel: CancellationToken) -> Self {
        Self { forwarder, cancel }
    }

    pub fn new_shared(forwarder: Arc<Forwarder>, cancel: CancellationToken) -> Arc<Self> {
        Arc::new(Self::new(forwarder, cancel))
    }

    // the routes that take the bearer token
    pub fn guarded(route: Arc<Self>) -> axum::Router {
        axum::Router::new()
            .route(Self::REGISTER, axum::routing::post(Self::register))
            .route(Self::POLL, axum::routing::get(Self::poll))
            .route(Self::REPLY, axum::routing::post(Self::reply))
            .route(Self::HEARTBEAT, axum::routing::post(Self::heartbeat))
            .route(Self::UNREGISTER, axum::routing::post(Self::unregister))
            .with_state(route)
    }

    pub async fn hello() -> Json<HubHello> {
        Json(HubHello::new())
    }

    async fn register(State(route): State<Arc<Self>>, Json(request): Json<RegisterRequest>) -> Response {
        let info = MemberInfo::new(InstanceId::new(request.id), request.pid, request.features.capabilities(), false);
        match route.registry().register(info) {
            Ok(()) => Json(HubHello::new()).into_response(),
            Err(RegistryError::Taken(_)) => StatusCode::CONFLICT.into_response(),
            Err(RegistryError::Full(_)) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
        }
    }

    async fn poll(State(route): State<Arc<Self>>, Query(member): Query<MemberRef>) -> Response {
        let id = InstanceId::new(member.id);
        let registry = Arc::clone(route.registry());
        let next = tokio::select! {
            _ = route.cancel.cancelled() => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
            next = registry.next(&id, Self::POLL_WAIT) => next,
        };
        match next {
            Ok(Some(forwarded)) => Json(forwarded).into_response(),
            Ok(None) => StatusCode::NO_CONTENT.into_response(),
            Err(_) => StatusCode::NOT_FOUND.into_response(),
        }
    }

    async fn reply(State(route): State<Arc<Self>>, Json(post): Json<ReplyPost>) -> StatusCode {
        route.registry().touch(&InstanceId::new(post.id));
        route.forwarder.complete(post.command, post.reply);
        StatusCode::NO_CONTENT
    }

    async fn heartbeat(State(route): State<Arc<Self>>, Json(member): Json<MemberRef>) -> StatusCode {
        if route.registry().touch(&InstanceId::new(member.id)) { StatusCode::NO_CONTENT } else { StatusCode::NOT_FOUND }
    }

    async fn unregister(State(route): State<Arc<Self>>, Json(member): Json<MemberRef>) -> StatusCode {
        route.registry().unregister(&InstanceId::new(member.id));
        StatusCode::NO_CONTENT
    }

    fn registry(&self) -> &Arc<MemberRegistry> {
        self.forwarder.registry()
    }
}
