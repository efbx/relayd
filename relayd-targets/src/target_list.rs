use std::sync::Arc;

use relayd_core::Target;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct TargetList {
    #[serde(default)]
    pub targets: Vec<Arc<Target>>,
}
