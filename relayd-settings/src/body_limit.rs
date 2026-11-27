use std::fmt::{self, Display, Formatter};

use bytesize::ByteSize;
use serde::{Deserialize, Deserializer, Serialize, de::Error as _};

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(into = "String")]
pub struct BodyLimit {
    amount: String,
    unit: &'static str,
    bytes: u64,
}

impl BodyLimit {
    pub const UNITS: [&'static str; 3] = ["KiB", "MiB", "GiB"];
    const INVALID: &'static str = "body limit: a number of KiB, MiB or GiB, 0 for headers only";

    pub fn new(amount: &str, unit: &str) -> Result<Self, &'static str> {
        let unit = Self::UNITS
            .into_iter()
            .find(|known| *known == unit)
            .ok_or(Self::INVALID)?;
        let amount = match amount.trim() {
            "" => "0",
            typed => typed,
        };
        let size: ByteSize = format!("{amount} {unit}")
            .parse()
            .map_err(|_| Self::INVALID)?;
        Ok(Self {
            amount: amount.to_owned(),
            unit,
            bytes: size.as_u64(),
        })
    }

    #[must_use]
    pub fn bytes(&self) -> u64 {
        self.bytes
    }

    #[must_use]
    pub fn into_parts(self) -> (String, &'static str) {
        (self.amount, self.unit)
    }
}

impl Default for BodyLimit {
    fn default() -> Self {
        Self {
            amount: "1".to_owned(),
            unit: "MiB",
            bytes: 1 << 20,
        }
    }
}

impl Display for BodyLimit {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} {}", self.amount, self.unit)
    }
}

impl TryFrom<String> for BodyLimit {
    type Error = &'static str;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        let (amount, unit) = text.trim().rsplit_once(' ').ok_or(Self::INVALID)?;
        Self::new(amount, unit)
    }
}

impl From<BodyLimit> for String {
    fn from(limit: BodyLimit) -> Self {
        limit.to_string()
    }
}

impl<'de> Deserialize<'de> for BodyLimit {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer)?
            .try_into()
            .map_err(D::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bytes(amount: &str, unit: &str) -> u64 {
        BodyLimit::new(amount, unit).unwrap().bytes()
    }

    #[test]
    fn the_default_is_one_mebibyte() {
        let limit = BodyLimit::default();
        assert_eq!(limit, BodyLimit::new("1", "MiB").unwrap());
        assert_eq!(limit.bytes(), 1_048_576);
    }

    #[test]
    fn each_unit_scales_by_1024() {
        assert_eq!(bytes("1", "KiB"), 1_024);
        assert_eq!(bytes("1", "MiB"), 1_048_576);
        assert_eq!(bytes("1", "GiB"), 1_073_741_824);
    }

    #[test]
    fn decimal_amounts_are_allowed() {
        assert_eq!(bytes("1.5", "MiB"), 1_572_864);
        assert_eq!(bytes("0.5", "KiB"), 512);
        assert_eq!(bytes(".25", "GiB"), 268_435_456);
    }

    #[test]
    fn zero_and_a_blank_amount_keep_no_body() {
        assert_eq!(bytes("0", "MiB"), 0);
        assert_eq!(bytes("0.0", "GiB"), 0);
        assert_eq!(bytes(" ", "KiB"), 0);
    }

    #[test]
    fn other_units_and_non_numbers_are_refused() {
        for (amount, unit) in [
            ("1", "MB"),
            ("1", "TiB"),
            ("1", ""),
            ("x", "MiB"),
            ("-1", "MiB"),
            ("1,5", "MiB"),
            ("1 Ki", "MiB"),
            ("1e3", "KiB"),
        ] {
            assert_eq!(
                BodyLimit::new(amount, unit),
                Err(BodyLimit::INVALID),
                "{amount:?} {unit:?}"
            );
        }
    }

    #[test]
    fn the_typed_amount_and_unit_are_kept_as_written() {
        let limit = BodyLimit::new(" 0.50 ", "GiB").unwrap();
        assert_eq!(limit.to_string(), "0.50 GiB");
        assert_eq!(limit.into_parts(), ("0.50".to_owned(), "GiB"));
    }

    #[test]
    fn it_is_stored_as_one_readable_string() {
        let limit = BodyLimit::new("1.5", "MiB").unwrap();
        let text = ron::to_string(&limit).unwrap();
        assert_eq!(text, "\"1.5 MiB\"");
        assert_eq!(ron::from_str::<BodyLimit>(&text), Ok(limit));
        assert!(ron::from_str::<BodyLimit>("\"1.5MiB\"").is_err());
        assert!(ron::from_str::<BodyLimit>("\"lots\"").is_err());
    }
}
