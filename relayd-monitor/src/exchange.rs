use std::borrow::Cow;

#[cfg(feature = "backend")]
use relayd_storage::Record;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct Exchange {
    pub id: u64,
    pub request: Message,
    pub response: Option<Message>,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct Message {
    pub headers: Vec<(String, Cow<'static, str>)>,
    pub body: Content,
    pub truncated: bool,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum Content {
    Empty,
    Text(String),
    Binary,
}

impl Exchange {
    pub const BODY_CAP: usize = 1 << 20;
}

#[cfg(feature = "backend")]
impl Content {
    pub(crate) fn from_bytes(bytes: Vec<u8>, truncated: bool) -> Self {
        if bytes.is_empty() {
            return Self::Empty;
        }
        match String::from_utf8(bytes) {
            Ok(text) => Self::Text(text),
            Err(error) if truncated && error.utf8_error().error_len().is_none() => {
                let valid = error.utf8_error().valid_up_to();
                let mut bytes = error.into_bytes();
                bytes.truncate(valid);
                String::from_utf8(bytes).map_or(Self::Binary, Self::Text)
            }
            Err(_) => Self::Binary,
        }
    }
}

#[cfg(feature = "backend")]
impl Record for Exchange {
    const TABLE: &'static str = "exchanges";
    const VERSION: u32 = 1;

    fn key(&self) -> u64 {
        self.id
    }
}
