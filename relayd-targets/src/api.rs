use dioxus_fullstack::ServerFnError;

use crate::TargetList;
#[cfg(any(feature = "server", feature = "standalone"))]
use crate::TargetStore;

#[sylk::server(handle: TargetStore)]
pub async fn targets() -> Result<TargetList, ServerFnError> {
    Ok(handle.read().clone())
}

#[sylk::server(handle: TargetStore)]
pub async fn save_targets(targets: TargetList) -> Result<(), ServerFnError> {
    handle
        .save(targets)
        .map_err(|error| ServerFnError::ServerError {
            message: error.message,
            code: 400,
            details: None,
        })
}
