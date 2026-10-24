use std::{
    borrow::Cow,
    fmt::{self, Debug, Formatter},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use axum::http::{
    HeaderMap, HeaderName,
    header::{AUTHORIZATION, COOKIE, PROXY_AUTHORIZATION, SET_COOKIE},
};
use relayd_core::Rule;

use crate::{Content, Direction, Exchange, Message};

#[derive(Clone, Debug)]
pub struct CaptureLimit(Arc<AtomicU64>);

impl CaptureLimit {
    #[must_use]
    pub fn new(count: u64) -> Self {
        Self(Arc::new(AtomicU64::new(count)))
    }

    pub(crate) fn set(&self, count: u64) {
        self.0.store(count, Ordering::Relaxed);
    }

    #[must_use]
    pub(crate) fn count(&self) -> u64 {
        self.0.load(Ordering::Relaxed)
    }
}

#[derive(Debug)]
pub struct Capture {
    request: Partial,
    response: Option<Partial>,
}

impl Capture {
    const REDACTED: [HeaderName; 5] = [
        AUTHORIZATION,
        PROXY_AUTHORIZATION,
        COOKIE,
        SET_COOKIE,
        HeaderName::from_static("x-api-key"),
    ];

    pub fn new(headers: &HeaderMap, rules: &[Rule]) -> Self {
        Self {
            request: Partial::new(headers, rules),
            response: None,
        }
    }

    pub fn answer(&mut self, headers: &HeaderMap) {
        self.response = Some(Partial::new(headers, &[]));
    }

    pub fn push(&mut self, direction: Direction, chunk: &[u8]) {
        match direction {
            Direction::Sent => self.request.push(chunk),
            Direction::Received => {
                if let Some(response) = &mut self.response {
                    response.push(chunk);
                }
            }
        }
    }

    pub fn into_exchange(self, id: u64) -> Exchange {
        Exchange {
            id,
            request: self.request.into_message(),
            response: self.response.map(Partial::into_message),
        }
    }

    fn shown(name: &HeaderName, value: &[u8], rules: &[Rule]) -> Cow<'static, str> {
        if Self::REDACTED.contains(name) || Self::injects(rules, name) {
            Cow::Borrowed("<redacted>")
        } else {
            Cow::Owned(String::from_utf8_lossy(value).into_owned())
        }
    }

    fn injects(rules: &[Rule], name: &HeaderName) -> bool {
        rules
            .iter()
            .any(|rule| matches!(rule, Rule::Set(set, _) if set == name))
    }
}

struct Partial {
    headers: Vec<(String, Cow<'static, str>)>,
    body: Vec<u8>,
    truncated: bool,
}

impl Partial {
    fn new(headers: &HeaderMap, rules: &[Rule]) -> Self {
        Self {
            headers: headers
                .iter()
                .map(|(name, value)| {
                    (
                        name.as_str().to_owned(),
                        Capture::shown(name, value.as_bytes(), rules),
                    )
                })
                .collect(),
            body: Vec::new(),
            truncated: false,
        }
    }

    fn push(&mut self, chunk: &[u8]) {
        let kept = chunk.len().min(Exchange::BODY_CAP - self.body.len());
        self.body.extend_from_slice(&chunk[..kept]);
        self.truncated |= kept < chunk.len();
    }

    fn into_message(self) -> Message {
        Message {
            headers: self.headers,
            body: Content::from_bytes(self.body, self.truncated),
            truncated: self.truncated,
        }
    }
}

impl Debug for Partial {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Partial")
            .field("headers", &self.headers.len())
            .field("body", &self.body.len())
            .field("truncated", &self.truncated)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use axum::http::HeaderValue;

    use super::*;

    fn headers(pairs: &[(&'static str, &'static str)]) -> HeaderMap {
        pairs
            .iter()
            .map(|(name, value)| {
                (
                    HeaderName::from_static(name),
                    HeaderValue::from_static(value),
                )
            })
            .collect()
    }

    fn captured(chunks: &[&[u8]]) -> Message {
        let mut capture = Capture::new(&HeaderMap::new(), &[]);
        for chunk in chunks {
            capture.push(Direction::Sent, chunk);
        }
        capture.into_exchange(1).request
    }

    #[test]
    fn credentials_are_redacted_and_other_headers_kept_in_order() {
        let capture = Capture::new(
            &headers(&[
                ("content-type", "application/json"),
                ("authorization", "Bearer real"),
                ("x-api-key", "k"),
                ("cookie", "session=1"),
            ]),
            &[],
        );
        let request = capture.into_exchange(1).request;
        assert_eq!(
            request.headers,
            [
                ("content-type".to_owned(), Cow::Borrowed("application/json")),
                ("authorization".to_owned(), Cow::Borrowed("<redacted>")),
                ("x-api-key".to_owned(), Cow::Borrowed("<redacted>")),
                ("cookie".to_owned(), Cow::Borrowed("<redacted>")),
            ]
        );
    }

    #[test]
    fn headers_a_set_rule_injects_are_redacted() {
        let rules = [Rule::try_from("set api-key real".to_owned()).unwrap()];
        let capture = Capture::new(&headers(&[("api-key", "real"), ("x-trace", "abc")]), &rules);
        assert_eq!(
            capture.into_exchange(1).request.headers,
            [
                ("api-key".to_owned(), Cow::Borrowed("<redacted>")),
                ("x-trace".to_owned(), Cow::Borrowed("abc")),
            ]
        );
    }

    #[test]
    fn response_headers_are_redacted_too() {
        let mut capture = Capture::new(&HeaderMap::new(), &[]);
        capture.answer(&headers(&[("set-cookie", "sid=1")]));
        assert_eq!(
            capture.into_exchange(1).response.unwrap().headers,
            [("set-cookie".to_owned(), Cow::Borrowed("<redacted>"))]
        );
    }

    #[test]
    fn text_bodies_are_kept_whole_and_empty_ones_stay_empty() {
        let text = captured(&[b"{\"a\":", b"1}"]);
        assert_eq!(text.body, Content::Text("{\"a\":1}".to_owned()));
        assert!(!text.truncated);
        assert_eq!(captured(&[]).body, Content::Empty);
    }

    #[test]
    fn binary_bodies_keep_no_contents() {
        assert_eq!(captured(&[&[0xff, 0xfe, 0x00]]).body, Content::Binary);
    }

    #[test]
    fn bodies_stop_at_the_cap_and_a_cut_character_is_dropped() {
        let filler = vec![b'a'; Exchange::BODY_CAP - 1];
        let message = captured(&[&filler, "é".as_bytes(), b"rest"]);
        assert!(message.truncated);
        let Content::Text(text) = message.body else {
            panic!("a truncated text body became {:?}", message.body);
        };
        assert_eq!(text.len(), Exchange::BODY_CAP - 1);
        assert!(text.bytes().all(|byte| byte == b'a'));
    }

    #[test]
    fn response_chunks_go_to_the_response_once_answered() {
        let mut capture = Capture::new(&HeaderMap::new(), &[]);
        capture.push(Direction::Received, b"early");
        capture.answer(&headers(&[("content-type", "text/plain")]));
        capture.push(Direction::Received, b"late");
        let exchange = capture.into_exchange(7);
        let response = exchange.response.unwrap();
        assert_eq!(exchange.id, 7);
        assert_eq!(response.body, Content::Text("late".to_owned()));
        assert_eq!(response.headers[0].0, "content-type");
    }

    #[test]
    fn debug_output_shows_sizes_not_contents() {
        let mut capture = Capture::new(&headers(&[("authorization", "Bearer real")]), &[]);
        capture.push(Direction::Sent, b"secret body");
        let shown = format!("{capture:?}");
        assert!(shown.contains("headers: 1") && shown.contains("body: 11"));
        assert!(!shown.contains("real") && !shown.contains("secret"));
    }

    #[test]
    fn the_limit_is_shared_between_clones() {
        let limit = CaptureLimit::new(10);
        let other = limit.clone();
        other.set(3);
        assert_eq!(limit.count(), 3);
    }
}
