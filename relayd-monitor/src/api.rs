#[cfg(any(feature = "server", feature = "standalone"))]
use crate::Monitor;
use dioxus_fullstack::{JsonEncoding, ServerFnError, Streaming};

use crate::{Entry, Exchange, Page};

#[sylk::get("/api/monitor/changes", monitor: Monitor)]
pub async fn changes() -> Result<Streaming<u64, JsonEncoding>, ServerFnError> {
    Ok(Streaming::new(monitor.changes()))
}

#[sylk::get("/api/monitor?page", monitor: Monitor)]
pub async fn entries(page: usize) -> Result<Page<Entry>, ServerFnError> {
    monitor
        .page(page)
        .map_err(|error| ServerFnError::ServerError {
            message: error.message,
            code: 500,
            details: None,
        })
}

#[sylk::get("/api/monitor/exchange?id", monitor: Monitor)]
pub async fn exchange(id: u64) -> Result<Option<Exchange>, ServerFnError> {
    monitor
        .exchange(id)
        .map_err(|error| ServerFnError::ServerError {
            message: error.message,
            code: 500,
            details: None,
        })
}

#[sylk::server(monitor: Monitor)]
pub async fn clear_history() -> Result<(), ServerFnError> {
    monitor.clear().map_err(|error| ServerFnError::ServerError {
        message: error.message,
        code: 500,
        details: None,
    })
}
