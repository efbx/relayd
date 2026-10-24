use std::fmt::{self, Debug, Display, Formatter};

use http::{HeaderName, HeaderValue};
use serde::{Deserialize, Serialize, Serializer};

#[derive(Clone, Deserialize)]
#[serde(try_from = "String")]
pub enum Rule {
    Set(HeaderName, HeaderValue),
    Remove(Vec<String>),
    Cors,
    Block,
    Key(String),
}

impl TryFrom<String> for Rule {
    type Error = String;

    fn try_from(line: String) -> Result<Self, String> {
        let (verb, rest) = line.split_once(' ').unwrap_or((line.as_str(), ""));
        match verb {
            "set" => {
                let (name, value) = rest
                    .split_once(' ')
                    .ok_or_else(|| format!("{line}: set needs a name and a value"))?;
                let name =
                    HeaderName::try_from(name).map_err(|_| format!("{line}: bad header name"))?;
                let value = HeaderValue::try_from(value.trim())
                    .map_err(|_| format!("{line}: bad header value"))?;
                Ok(Self::Set(name, value))
            }
            "remove" if !rest.trim().is_empty() => Ok(Self::Remove(
                rest.split_whitespace()
                    .map(str::to_ascii_lowercase)
                    .collect(),
            )),
            "cors" => Ok(Self::Cors),
            "block" => Ok(Self::Block),
            "key" if !rest.trim().is_empty() => Ok(Self::Key(rest.trim().to_owned())),
            "key" => Err(format!("{line}: key needs a secret")),
            _ => Err(format!("{line}: unknown rule")),
        }
    }
}

impl Display for Rule {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Set(name, value) => {
                write!(
                    formatter,
                    "set {name} {}",
                    str::from_utf8(value.as_bytes()).unwrap_or_default()
                )
            }
            Self::Remove(patterns) => {
                formatter.write_str("remove")?;
                patterns
                    .iter()
                    .try_for_each(|pattern| write!(formatter, " {pattern}"))
            }
            Self::Cors => formatter.write_str("cors"),
            Self::Block => formatter.write_str("block"),
            Self::Key(secret) => write!(formatter, "key {secret}"),
        }
    }
}

impl Debug for Rule {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Set(name, _) => formatter
                .debug_tuple("Set")
                .field(name)
                .field(&"<redacted>")
                .finish(),
            Self::Remove(patterns) => formatter.debug_tuple("Remove").field(patterns).finish(),
            Self::Cors => formatter.write_str("Cors"),
            Self::Block => formatter.write_str("Block"),
            Self::Key(_) => formatter.debug_tuple("Key").field(&"<redacted>").finish(),
        }
    }
}

impl Serialize for Rule {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_takes_a_name_and_the_rest_as_value() {
        let Ok(Rule::Set(name, value)) = Rule::try_from("set authorization Bearer a b".to_owned())
        else {
            panic!("set did not parse");
        };
        assert_eq!(name, "authorization");
        assert_eq!(value, "Bearer a b");
        assert_eq!(
            Rule::try_from("set authorization".to_owned())
                .err()
                .unwrap(),
            "set authorization: set needs a name and a value"
        );
    }

    #[test]
    fn remove_lowercases_every_name() {
        let Ok(Rule::Remove(names)) = Rule::try_from("remove Origin CF-*".to_owned()) else {
            panic!("remove did not parse");
        };
        assert_eq!(names, ["origin", "cf-*"]);
        assert!(
            Rule::try_from("remove".to_owned())
                .err()
                .unwrap()
                .ends_with("unknown rule")
        );
    }

    #[test]
    fn key_keeps_its_secret_trimmed() {
        let Ok(Rule::Key(secret)) = Rule::try_from("key  hunter2 ".to_owned()) else {
            panic!("key did not parse");
        };
        assert_eq!(secret, "hunter2");
        assert_eq!(
            Rule::try_from("key".to_owned()).err().unwrap(),
            "key: key needs a secret"
        );
    }

    #[test]
    fn bare_verbs_and_unknown_ones() {
        assert!(matches!(Rule::try_from("cors".to_owned()), Ok(Rule::Cors)));
        assert!(matches!(
            Rule::try_from("block".to_owned()),
            Ok(Rule::Block)
        ));
        assert_eq!(
            Rule::try_from("frob x".to_owned()).err().unwrap(),
            "frob x: unknown rule"
        );
    }

    #[test]
    fn text_round_trips() {
        for line in [
            "set user-agent node-fetch",
            "set x-title café",
            "remove origin cf-*",
            "cors",
            "block",
            "key hunter2",
        ] {
            assert_eq!(Rule::try_from(line.to_owned()).unwrap().to_string(), line);
        }
    }
}
