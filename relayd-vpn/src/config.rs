use std::path::PathBuf;

use relayd_core::DataDir;

#[derive(Clone, Debug)]
pub struct VpnConfig {
    pub dir: PathBuf,
}

#[sylk::resource]
fn vpn_config(dir: DataDir) -> VpnConfig {
    VpnConfig {
        dir: dir.0.join("vpn"),
    }
}
