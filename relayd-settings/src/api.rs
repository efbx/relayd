use dioxus_fullstack::ServerFnError;

use crate::Settings;
#[cfg(any(feature = "server", feature = "standalone"))]
use crate::SettingsStore;

#[sylk::server(handle: SettingsStore)]
pub async fn settings() -> Result<Settings, ServerFnError> {
    Ok(handle.current())
}

#[sylk::server(handle: SettingsStore)]
pub async fn save_settings(settings: Settings) -> Result<(), ServerFnError> {
    handle
        .save(settings)
        .map_err(|error| ServerFnError::ServerError {
            message: error.message,
            code: 400,
            details: None,
        })
}

#[sylk::server(handle: SettingsStore)]
pub async fn restart() -> Result<(), ServerFnError> {
    handle.restart();
    Ok(())
}
