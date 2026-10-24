use std::{
    fmt::{self, Debug, Formatter},
    sync::Arc,
};

use futures_util::Stream;
use sylk::Pack;

use crate::{TunnelConfig, TunnelMode, supervisor::Supervisor};

#[derive(Clone, Pack)]
pub struct Tunnel {
    config: TunnelConfig,
    #[pack(func = Supervisor::spawn(&config))]
    supervisor: Arc<Supervisor>,
}

impl Debug for Tunnel {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Tunnel")
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

impl Tunnel {
    #[must_use]
    pub fn current_url(&self) -> String {
        self.supervisor.current_url()
    }

    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.supervisor.is_enabled()
    }

    pub fn request_restart(&self) {
        self.supervisor.request_restart();
    }

    pub fn reconfigure(&self, mode: TunnelMode) {
        self.supervisor
            .reconfigure(self.config.target.clone(), mode);
    }

    pub fn url_updates(&self) -> impl Stream<Item = String> + Send + 'static {
        self.supervisor.url_updates()
    }
}

#[cfg(test)]
mod tests {
    use sylk::Resources;

    use super::*;

    #[tokio::test]
    async fn an_off_tunnel_runs_nothing_and_has_no_url() {
        let mut resources = Resources::new();
        resources.insert(TunnelConfig {
            target: String::from("http://127.0.0.1:8585"),
            mode: TunnelMode::Off,
        });
        let tunnel = Tunnel::build(&resources).unwrap();
        assert!(!tunnel.is_enabled());
        assert_eq!(tunnel.current_url(), "");
        tunnel.reconfigure(TunnelMode::Off);
        assert!(!tunnel.is_enabled());
    }
}
