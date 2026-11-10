use std::sync::Arc;

use axum::extract::{Request, State};
use axum::http::{StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use crate::token::TokenStore;

// Rejects every request whose `Authorization` is not `Bearer <token>` with a token of the `TokenStore`.
pub struct BearerGuard {
    tokens: Arc<TokenStore>,
}

impl BearerGuard {
    const PREFIX: &'static [u8] = b"Bearer ";

    pub fn new(tokens: Arc<TokenStore>) -> Self {
        Self { tokens }
    }

    // true when the header value is `Bearer <token>` and the store holds the token (compared in constant time)
    pub fn allows(&self, authorization: Option<&[u8]>) -> bool {
        let Some(token) = authorization.and_then(|value| value.strip_prefix(Self::PREFIX)) else {
            return false;
        };
        std::str::from_utf8(token).is_ok_and(|token| self.tokens.contains(token))
    }

    // the axum middleware: delegates to `allows`
    pub async fn check(State(guard): State<Arc<Self>>, request: Request, next: Next) -> Response {
        let authorization = request.headers().get(header::AUTHORIZATION).map(|value| value.as_bytes());
        if guard.allows(authorization) { next.run(request).await } else { StatusCode::UNAUTHORIZED.into_response() }
    }
}
