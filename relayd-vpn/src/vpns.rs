use std::{
    fmt::{self, Debug, Formatter},
    net::SocketAddr,
    path::Path,
    sync::Arc,
};

use sylk::Pack;

use crate::{VpnConfig, VpnInfo, supervisor::Supervisor};

#[derive(Clone, Pack)]
pub struct Vpns {
    config: VpnConfig,
    #[pack(func = Supervisor::spawn(&config))]
    supervisor: Arc<Supervisor>,
}

impl Debug for Vpns {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Vpns")
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

impl Vpns {
    #[must_use]
    pub fn dir(&self) -> &Path {
        &self.config.dir
    }

    #[must_use]
    pub fn list(&self) -> Vec<VpnInfo> {
        self.supervisor.list()
    }

    pub fn add(&self, name: String, config: String) -> Result<(), String> {
        self.supervisor.add(name, config)
    }

    pub fn remove(&self, name: &str) -> Result<(), String> {
        self.supervisor.remove(name)
    }

    #[must_use]
    pub fn socks_address(&self, name: &str) -> Option<SocketAddr> {
        self.supervisor.socks_address(name)
    }
}

#[cfg(test)]
mod tests {
    use sylk::Resources;

    use super::*;

    #[tokio::test]
    async fn an_empty_directory_lists_no_vpns() {
        let dir = std::env::temp_dir().join(format!("relayd-vpns-{}", std::process::id()));
        let mut resources = Resources::new();
        resources.insert(VpnConfig { dir: dir.clone() });
        let vpns = Vpns::build(&resources).unwrap();
        assert_eq!(vpns.dir(), dir);
        assert_eq!(vpns.list(), [] as [VpnInfo; 0]);
        assert_eq!(vpns.socks_address("Warp"), None);
    }
}
