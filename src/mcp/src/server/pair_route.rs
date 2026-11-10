use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use axum::body::Bytes;
use axum::extract::State;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};

use crate::dto::{PairReply, PairRequest};
use crate::port::HeadlessPort;
use crate::token::TokenStore;

// `POST /pair`: asks the person through the port, and on Allow issues a token into the store. No bearer check. Allow
// `200 {"token":...}`, deny or no answer `403`, a request while another waits for its answer `409`, a bad body `400`.
pub struct PairRoute<P: HeadlessPort> {
    port: Arc<P>,
    tokens: Arc<TokenStore>,
    asking: AtomicBool,
}

impl<P: HeadlessPort> PairRoute<P> {
    pub const PATH: &'static str = "/pair";

    pub fn new(port: Arc<P>, tokens: Arc<TokenStore>) -> Self {
        Self { port, tokens, asking: AtomicBool::new(false) }
    }

    pub fn new_shared(port: Arc<P>, tokens: Arc<TokenStore>) -> Arc<Self> {
        Arc::new(Self::new(port, tokens))
    }

    // the axum handler: the blocking work runs in `spawn_blocking`
    pub async fn handle(State(route): State<Arc<Self>>, body: Bytes) -> Response {
        match tokio::task::spawn_blocking(move || route.answer(&body)).await {
            Ok(response) => response,
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }

    // the reply to one pair body. Blocks while the person decides
    pub fn answer(&self, body: &[u8]) -> Response {
        let request = match serde_json::from_slice::<PairRequest>(body) {
            Ok(request) if request.is_valid() => request,
            _ => return StatusCode::BAD_REQUEST.into_response(),
        };
        if self.asking.swap(true, Ordering::SeqCst) {
            return StatusCode::CONFLICT.into_response();
        }
        let port = &self.port;
        let allowed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| port.approve_pairing(&request.client)))
            .unwrap_or(false);
        self.asking.store(false, Ordering::SeqCst);
        if !allowed {
            return StatusCode::FORBIDDEN.into_response();
        }
        match self.tokens.issue(&request.client) {
            Ok(token) => Self::json(&PairReply { token }),
            Err(error) => {
                curia::error!("the new token was not saved", { error: error.to_string() });
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }

    fn json(reply: &PairReply) -> Response {
        match serde_json::to_string(reply) {
            Ok(text) => (StatusCode::OK, [(header::CONTENT_TYPE, "application/json")], text).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}
