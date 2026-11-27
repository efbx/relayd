use std::net::{Ipv4Addr, SocketAddr};

#[cfg(feature = "backend")]
use relayd_storage::Retention;
use relayd_tunnel::TunnelMode;
use serde::{Deserialize, Serialize};

use crate::BodyLimit;

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(default)]
pub struct Settings {
    pub listen: SocketAddr,
    pub tunnel: TunnelMode,
    pub idle_timeout: u64,
    pub history: u64,
    pub history_prune_every: u64,
    pub captures: u64,
    pub captures_prune_every: u64,
    pub captures_body: BodyLimit,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            listen: SocketAddr::from((Ipv4Addr::LOCALHOST, 8585)),
            tunnel: TunnelMode::default(),
            idle_timeout: 0,
            history: 10_000,
            history_prune_every: 256,
            captures: 10,
            captures_prune_every: 5,
            captures_body: BodyLimit::default(),
        }
    }
}

#[cfg(feature = "backend")]
impl Settings {
    pub(crate) fn history_retention(&self) -> Retention {
        Retention {
            keep: self.history,
            every: self.history_prune_every,
        }
    }

    pub(crate) fn captures_retention(&self) -> Retention {
        Retention {
            keep: self.captures,
            every: self.captures_prune_every,
        }
    }

    pub(crate) fn body_cap(&self) -> usize {
        usize::try_from(self.captures_body.bytes()).unwrap_or(usize::MAX)
    }
}

#[cfg(test)]
mod tests {
    use relayd_tunnel::TunnelMode;

    use super::*;

    #[test]
    fn defaults_are_a_local_proxy_and_a_quick_tunnel() {
        let settings = Settings::default();
        assert_eq!(settings.listen.to_string(), "127.0.0.1:8585");
        assert_eq!(settings.tunnel, TunnelMode::Quick);
        assert_eq!((settings.idle_timeout, settings.captures), (0, 10));
    }

    #[test]
    fn an_old_file_with_an_admin_address_and_no_tunnel_still_loads() {
        let settings: Settings =
            ron::from_str("(listen: \"127.0.0.1:9000\", admin: \"0.0.0.0:8080\")").unwrap();
        assert_eq!(settings.listen.port(), 9000);
        assert_eq!(settings.tunnel, TunnelMode::Quick);
        assert_eq!((settings.idle_timeout, settings.captures), (0, 10));
    }

    #[test]
    fn history_and_contents_default_to_a_size_and_a_batch() {
        let settings = Settings::default();
        assert_eq!(
            settings.history_retention(),
            Retention {
                keep: 10_000,
                every: 256
            }
        );
        assert_eq!(
            settings.captures_retention(),
            Retention { keep: 10, every: 5 }
        );
    }

    #[test]
    fn a_file_from_before_the_limits_takes_their_defaults() {
        let settings: Settings = ron::from_str("(captures: 3)").unwrap();
        assert_eq!(
            settings.captures_retention(),
            Retention { keep: 3, every: 5 }
        );
        assert_eq!(
            (settings.history, settings.history_prune_every),
            (10_000, 256)
        );
        assert_eq!(settings.body_cap(), 1 << 20);
    }

    #[test]
    fn the_body_limit_reads_from_ron_with_its_unit() {
        let settings: Settings = ron::from_str("(captures_body: \"1.5 MiB\")").unwrap();
        assert_eq!(settings.body_cap(), 1_572_864);
        let off: Settings = ron::from_str("(captures_body: \"0 GiB\")").unwrap();
        assert_eq!(off.body_cap(), 0);
        assert!(ron::from_str::<Settings>("(captures_body: \"1 MB\")").is_err());
    }

    #[test]
    fn saved_settings_read_back_the_same() {
        let settings = Settings {
            captures_body: BodyLimit::new("0.25", "GiB").unwrap(),
            ..Settings::default()
        };
        let text = ron::to_string(&settings).unwrap();
        assert_eq!(ron::from_str::<Settings>(&text), Ok(settings));
    }

    #[test]
    fn every_tunnel_mode_reads_from_ron() {
        let off: Settings = ron::from_str("(tunnel: Off)").unwrap();
        let domain: Settings = ron::from_str("(tunnel: Domain(token: \"abc\"))").unwrap();
        assert_eq!(off.tunnel, TunnelMode::Off);
        assert_eq!(
            domain.tunnel,
            TunnelMode::Domain {
                token: "abc".into()
            }
        );
    }
}
