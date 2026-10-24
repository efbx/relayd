use std::{
    fmt::{self, Debug, Formatter},
    sync::Arc,
};

use http::{HeaderMap, Method, StatusCode};

use crate::{Error, Target};

pub type Body = axum::body::Body;
pub type Reply = axum::response::Response;

#[derive(Debug)]
pub struct Ctx {
    pub id: u64,
    pub method: Method,
    pub path: Arc<str>,
    pub target: Arc<Target>,
}

pub struct Outgoing {
    pub method: Method,
    pub url: String,
    pub headers: HeaderMap,
}

impl Debug for Outgoing {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Outgoing")
            .field("method", &self.method)
            .field("url", &self.url)
            .field("headers", &"<redacted>")
            .finish()
    }
}

pub struct Incoming {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: Body,
}

impl Debug for Incoming {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Incoming")
            .field("status", &self.status)
            .field("headers", &"<redacted>")
            .finish_non_exhaustive()
    }
}

impl Incoming {
    #[must_use]
    pub fn from_reply(reply: Reply) -> Self {
        let (parts, body) = reply.into_parts();
        Self {
            status: parts.status,
            headers: parts.headers,
            body,
        }
    }

    #[must_use]
    pub fn from_error(error: Error) -> Self {
        Self {
            status: error.status,
            headers: HeaderMap::new(),
            body: Body::from(error.message),
        }
    }

    #[must_use]
    pub fn into_reply(self) -> Reply {
        let mut reply = Reply::new(self.body);
        *reply.status_mut() = self.status;
        *reply.headers_mut() = self.headers;
        reply
    }
}
