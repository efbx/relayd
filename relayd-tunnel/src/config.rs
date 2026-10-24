use crate::TunnelMode;

#[derive(Clone, Debug)]
pub struct TunnelConfig {
    pub target: String,
    pub mode: TunnelMode,
}
