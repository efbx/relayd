#[cfg(feature = "backend")]
use std::time::SystemTime;
use std::{
    borrow::Cow,
    fmt::{self, Display, Formatter},
    sync::Arc,
};

use humantime::FormattedDuration;
use serde::{
    Deserialize, Deserializer, Serialize,
    de::{self, Visitor},
};
#[cfg(feature = "backend")]
use {relayd_core::Ctx, relayd_storage::Record};

use crate::{Human, Label};

#[derive(Clone, Copy, Debug, PartialEq, Deserialize, Serialize)]
pub enum Stage {
    Received,
    Forwarded,
    Answered,
    Done,
    Failed,
    Cancelled,
}

#[cfg(feature = "backend")]
#[derive(Clone, Copy, Debug)]
pub enum Direction {
    Sent,
    Received,
}

#[cfg(feature = "backend")]
#[derive(Clone, Copy, Debug)]
pub enum Event {
    RequestSent,
    Answered(u16, Option<u64>),
    ReplyFinished,
    ReplyBroken,
    Cancelled,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct Entry {
    pub id: u64,
    #[serde(deserialize_with = "Entry::decode_method")]
    pub method: Cow<'static, str>,
    pub path: Arc<str>,
    pub prefix: Arc<str>,
    pub stage: Stage,
    pub status: Option<u16>,
    pub started: u64,
    pub elapsed: u64,
    pub sent: u64,
    pub received: u64,
    #[serde(skip)]
    pub(crate) expected: Option<u64>,
}

impl Entry {
    const METHODS: [&'static str; 9] = [
        "GET", "POST", "PUT", "DELETE", "HEAD", "OPTIONS", "CONNECT", "PATCH", "TRACE",
    ];

    fn method_name(name: &str) -> Cow<'static, str> {
        Self::METHODS
            .into_iter()
            .find(|known| *known == name)
            .map_or_else(|| Cow::Owned(name.to_owned()), Cow::Borrowed)
    }

    fn decode_method<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Cow<'static, str>, D::Error> {
        deserializer.deserialize_str(MethodVisitor)
    }

    #[must_use]
    pub fn finished(&self) -> bool {
        matches!(self.stage, Stage::Done | Stage::Failed | Stage::Cancelled)
    }

    #[must_use]
    pub fn status_label(&self) -> Label<u16> {
        Label(self.status)
    }

    #[must_use]
    pub fn elapsed_label(&self) -> Label<FormattedDuration> {
        Label(self.status.map(|_| Human::duration(self.elapsed)))
    }
}

impl Display for Entry {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} {} {} {} {} {} up {} down",
            self.id,
            self.method,
            self.path,
            self.status_label(),
            self.elapsed_label(),
            Human::size(self.sent),
            Human::size(self.received)
        )
    }
}

struct MethodVisitor;

impl Visitor<'_> for MethodVisitor {
    type Value = Cow<'static, str>;

    fn expecting(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("an HTTP method")
    }

    fn visit_str<E: de::Error>(self, method: &str) -> Result<Self::Value, E> {
        Ok(Entry::method_name(method))
    }
}

#[cfg(feature = "backend")]
impl Record for Entry {
    const TABLE: &'static str = "requests";
    const VERSION: u32 = 1;

    fn key(&self) -> u64 {
        self.id
    }
}

#[cfg(feature = "backend")]
impl Entry {
    #[must_use]
    pub(crate) fn new(ctx: &Ctx) -> Self {
        Self {
            id: ctx.id,
            method: Self::method_name(ctx.method.as_str()),
            path: Arc::clone(&ctx.path),
            prefix: Arc::clone(&ctx.target.prefix),
            stage: Stage::Received,
            status: None,
            started: Self::elapsed(),
            elapsed: 0,
            sent: 0,
            received: 0,
            expected: None,
        }
    }

    pub(crate) fn update_length(&mut self, direction: Direction, length: u64) {
        match direction {
            Direction::Sent => self.sent += length,
            Direction::Received => {
                self.received += length;
                if self.expected == Some(self.received) {
                    self.apply(Event::ReplyFinished);
                }
            }
        }
    }

    pub(crate) fn apply(&mut self, event: Event) {
        let stage = match event {
            Event::RequestSent if self.stage == Stage::Received => Stage::Forwarded,
            Event::RequestSent => return,
            Event::Answered(status, expected) => {
                self.status = Some(status);
                self.expected = expected;
                Stage::Answered
            }
            Event::ReplyFinished if self.status.is_some_and(|status| status >= 500) => {
                Stage::Failed
            }
            Event::ReplyFinished => Stage::Done,
            Event::ReplyBroken => Stage::Failed,
            Event::Cancelled => Stage::Cancelled,
        };
        self.stage = stage;
        self.elapsed = Self::elapsed().saturating_sub(self.started);
    }

    fn elapsed() -> u64 {
        match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
            Ok(duration) => u64::try_from(duration.as_millis()).unwrap_or(u64::MAX),
            Err(_) => 0,
        }
    }
}

#[cfg(all(test, feature = "backend"))]
mod tests {
    use super::*;

    fn entry() -> Entry {
        Entry {
            id: 1,
            method: "POST".into(),
            path: "/sm/x".into(),
            prefix: "/sm".into(),
            stage: Stage::Received,
            status: None,
            started: Entry::elapsed(),
            elapsed: 0,
            sent: 0,
            received: 0,
            expected: None,
        }
    }

    #[test]
    fn bytes_add_up_per_direction() {
        let mut entry = entry();
        entry.update_length(Direction::Sent, 10);
        entry.update_length(Direction::Sent, 5);
        entry.update_length(Direction::Received, 7);
        assert_eq!((entry.sent, entry.received), (15, 7));
    }

    fn after(events: &[Event]) -> Entry {
        let mut entry = entry();
        for event in events {
            entry.apply(*event);
        }
        entry
    }

    #[test]
    fn stages_follow_the_request() {
        let mut entry = after(&[Event::RequestSent]);
        assert_eq!(entry.stage, Stage::Forwarded);
        entry.apply(Event::Answered(200, None));
        assert!(entry.stage == Stage::Answered && entry.status == Some(200));
        entry.apply(Event::RequestSent);
        assert_eq!(entry.stage, Stage::Answered);
        entry.apply(Event::ReplyFinished);
        assert_eq!(entry.stage, Stage::Done);
    }

    #[test]
    fn a_server_error_fails_once_its_body_is_through() {
        let mut entry = after(&[Event::Answered(503, None)]);
        entry.update_length(Direction::Received, 9);
        assert_eq!(entry.stage, Stage::Answered);
        entry.apply(Event::ReplyFinished);
        assert!(entry.stage == Stage::Failed && entry.received == 9);
    }

    #[test]
    fn broken_and_cancelled_replies_are_finished() {
        let broken = after(&[Event::Answered(200, None), Event::ReplyBroken]);
        let cancelled = after(&[Event::RequestSent, Event::Cancelled]);
        assert!(broken.stage == Stage::Failed && broken.finished());
        assert!(cancelled.stage == Stage::Cancelled && cancelled.finished());
    }

    #[test]
    fn a_reply_of_known_size_finishes_with_its_last_byte() {
        let mut done = after(&[Event::Answered(200, Some(4))]);
        done.update_length(Direction::Received, 3);
        assert_eq!(done.stage, Stage::Answered);
        done.update_length(Direction::Received, 1);
        assert_eq!(done.stage, Stage::Done);
        let mut failed = after(&[Event::Answered(500, Some(33))]);
        failed.update_length(Direction::Received, 33);
        assert_eq!(failed.stage, Stage::Failed);
    }
}
