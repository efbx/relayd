use dioxus::prelude::*;
use relayd_core::Target;
use serde::Serialize;
use serde_json::Value;

#[derive(Store, Clone, PartialEq, Default, Serialize)]
pub(crate) struct TargetDraft {
    pub(crate) prefix: String,
    pub(crate) upstream: String,
    pub(crate) rules: Vec<String>,
    pub(crate) vpn: String,
}

impl TargetDraft {
    pub(super) fn from_target(target: Target) -> Self {
        Self {
            prefix: String::from(&*target.prefix),
            upstream: target.upstream,
            rules: target.rules.iter().map(ToString::to_string).collect(),
            vpn: target.vpn.unwrap_or_default(),
        }
    }

    pub(super) fn parse(&self) -> Result<Target, String> {
        let mut value = serde_json::to_value(self).expect("draft serialize");
        if self.vpn.is_empty() {
            value["vpn"] = Value::Null;
        }
        serde_json::from_value(value).map_err(|error| error.to_string())
    }
}
