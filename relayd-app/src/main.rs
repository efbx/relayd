#[cfg(feature = "web")]
fn main() {
    dioxus::launch(relayd_ui::App);
}

#[cfg(feature = "server")]
fn main() {
    use std::path::PathBuf;

    use anyhow::anyhow;
    use relayd_core::DataDir;
    use {
        relayd_monitor as _, relayd_proxy as _, relayd_settings as _, relayd_targets as _,
        relayd_tunnel as _, relayd_vpn as _,
    };

    sylk::serve(
        async |resources| {
            rustls_graviola::default_provider()
                .install_default()
                .map_err(|_| anyhow!("a tls provider is already installed"))?;
            resources.insert(DataDir(PathBuf::from("data")));
            Ok(())
        },
        sylk::client_rendered,
    );
}

#[cfg(feature = "standalone")]
fn main() {
    use anyhow::{Context, anyhow};
    use dioxus::LaunchBuilder;
    use relayd_core::DataDir;
    use relayd_ui::TitleBarWindow;
    use {
        relayd_monitor as _, relayd_proxy as _, relayd_settings as _, relayd_targets as _,
        relayd_tunnel as _, relayd_vpn as _,
    };

    dioxus::logger::initialize_default();
    sylk::launch(
        async |resources| {
            rustls_graviola::default_provider()
                .install_default()
                .map_err(|_| anyhow!("a tls provider is already installed"))?;
            let data = dirs::data_dir().context("this system has no data folder")?;
            resources.insert(DataDir(data.join("RelayD")));
            Ok(())
        },
        LaunchBuilder::desktop().with_cfg(TitleBarWindow::config(
            "RelayD",
            (1280.0, 840.0),
            (980.0, 640.0),
        )),
        relayd_ui::App,
    );
}
