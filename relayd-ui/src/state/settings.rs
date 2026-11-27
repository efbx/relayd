use std::net::SocketAddr;

use relayd_settings::{BodyLimit, Settings};
use relayd_tunnel::TunnelMode;

#[derive(Clone, PartialEq)]
pub(crate) struct SettingsDraft {
    pub(crate) listen: String,
    pub(crate) tunnel: String,
    pub(crate) token: String,
    pub(crate) idle_timeout: String,
    pub(crate) history: String,
    pub(crate) history_prune_every: String,
    pub(crate) captures: String,
    pub(crate) captures_prune_every: String,
    pub(crate) captures_body: String,
    pub(crate) captures_body_unit: String,
}

impl SettingsDraft {
    pub(crate) const OFF: &'static str = "Off";
    pub(crate) const QUICK: &'static str = "Quick";
    pub(crate) const DOMAIN: &'static str = "Own domain";

    pub(super) fn from_settings(settings: Settings) -> Self {
        let (tunnel, token) = match settings.tunnel {
            TunnelMode::Off => (String::new(), String::new()),
            TunnelMode::Quick => (Self::QUICK.to_owned(), String::new()),
            TunnelMode::Domain { token } => (Self::DOMAIN.to_owned(), token),
        };
        let (captures_body, unit) = settings.captures_body.into_parts();
        Self {
            listen: settings.listen.to_string(),
            tunnel,
            token,
            idle_timeout: settings.idle_timeout.to_string(),
            history: settings.history.to_string(),
            history_prune_every: settings.history_prune_every.to_string(),
            captures: settings.captures.to_string(),
            captures_prune_every: settings.captures_prune_every.to_string(),
            captures_body,
            captures_body_unit: unit.to_owned(),
        }
    }

    pub(super) fn parse(&self) -> Result<Settings, String> {
        Ok(Settings {
            listen: Self::address("listen", &self.listen)?,
            tunnel: self.tunnel_mode()?,
            idle_timeout: Self::whole("idle timeout", &self.idle_timeout, "seconds")?,
            history: Self::whole("history", &self.history, "requests")?,
            history_prune_every: Self::batch("history prune every", &self.history_prune_every)?,
            captures: Self::whole("stored contents", &self.captures, "requests")?,
            captures_prune_every: Self::batch(
                "stored contents prune every",
                &self.captures_prune_every,
            )?,
            captures_body: BodyLimit::new(&self.captures_body, &self.captures_body_unit)?,
        })
    }

    pub(crate) fn wants_token(&self) -> bool {
        self.tunnel == Self::DOMAIN
    }

    fn tunnel_mode(&self) -> Result<TunnelMode, String> {
        match self.tunnel.as_str() {
            "" => Ok(TunnelMode::Off),
            Self::QUICK => Ok(TunnelMode::Quick),
            Self::DOMAIN if self.token.trim().is_empty() => {
                Err("tunnel: own domain needs the tunnel token".to_owned())
            }
            Self::DOMAIN => Ok(TunnelMode::Domain {
                token: self.token.trim().to_owned(),
            }),
            other => Err(format!("tunnel: unknown mode {other}")),
        }
    }

    fn address(name: &str, text: &str) -> Result<SocketAddr, String> {
        text.trim()
            .parse()
            .map_err(|_| format!("{name}: invalid socket address"))
    }

    fn whole(name: &str, text: &str, unit: &str) -> Result<u64, String> {
        match text.trim() {
            "" => Ok(0),
            number => number
                .parse()
                .map_err(|_| format!("{name}: whole {unit}, 0 for off")),
        }
    }

    fn batch(name: &str, text: &str) -> Result<u64, String> {
        text.trim()
            .parse()
            .ok()
            .filter(|every| *every > 0)
            .ok_or_else(|| format!("{name}: whole requests, at least 1"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draft() -> SettingsDraft {
        SettingsDraft::from_settings(Settings::default())
    }

    #[test]
    fn the_defaults_survive_a_round_trip() {
        assert_eq!(draft().parse(), Ok(Settings::default()));
    }

    #[test]
    fn a_blank_size_reads_as_off() {
        let draft = SettingsDraft {
            history: String::new(),
            captures: " ".to_owned(),
            ..draft()
        };
        let parsed = draft.parse().unwrap();
        assert_eq!((parsed.history, parsed.captures), (0, 0));
    }

    #[test]
    fn the_body_limit_takes_decimals_and_a_unit() {
        let draft = SettingsDraft {
            captures_body: "1.5".to_owned(),
            captures_body_unit: "GiB".to_owned(),
            ..draft()
        };
        let parsed = draft.parse().unwrap();
        assert_eq!(parsed.captures_body.bytes(), 1_610_612_736);
        assert_eq!(SettingsDraft::from_settings(parsed).parse(), draft.parse());
    }

    #[test]
    fn a_blank_body_limit_reads_as_headers_only() {
        let draft = SettingsDraft {
            captures_body: String::new(),
            ..draft()
        };
        assert_eq!(draft.parse().unwrap().captures_body.bytes(), 0);
    }

    #[test]
    fn a_body_limit_must_be_a_number_of_a_known_unit() {
        for (amount, unit) in [("x", "MiB"), ("-1", "MiB"), ("1", "MB"), ("1", "")] {
            let draft = SettingsDraft {
                captures_body: amount.to_owned(),
                captures_body_unit: unit.to_owned(),
                ..draft()
            };
            assert_eq!(
                draft.parse(),
                Err("body limit: a number of KiB, MiB or GiB, 0 for headers only".to_owned())
            );
        }
    }

    #[test]
    fn a_batch_must_be_at_least_one() {
        for every in ["", "0", "x"] {
            let draft = SettingsDraft {
                history_prune_every: every.to_owned(),
                ..draft()
            };
            assert_eq!(
                draft.parse(),
                Err("history prune every: whole requests, at least 1".to_owned())
            );
        }
    }
}
