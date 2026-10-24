use std::fmt::{self, Debug, Formatter};

use serde::{Deserialize, Serialize};

#[derive(Clone, Default, PartialEq, Deserialize, Serialize)]
pub enum TunnelMode {
    Off,
    #[default]
    Quick,
    Domain {
        token: String,
    },
}

impl TunnelMode {
    #[must_use]
    pub fn enabled(&self) -> bool {
        !matches!(self, Self::Off)
    }
}

impl Debug for TunnelMode {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Off => formatter.write_str("Off"),
            Self::Quick => formatter.write_str("Quick"),
            Self::Domain { .. } => formatter
                .debug_struct("Domain")
                .field("token", &"<redacted>")
                .finish(),
        }
    }
}
