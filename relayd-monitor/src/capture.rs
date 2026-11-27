use std::{
    borrow::Cow,
    fmt::{self, Debug, Formatter},
};

use axum::http::{
    HeaderMap, HeaderName,
    header::{AUTHORIZATION, COOKIE, PROXY_AUTHORIZATION, SET_COOKIE},
};
use relayd_core::Rule;

use crate::{Content, Direction, Exchange, Message};

#[derive(Debug)]
pub struct Capture {
    request: Partial,
    response: Option<Partial>,
    cap: usize,
}

impl Capture {
    const REDACTED: [HeaderName; 5] = [
        AUTHORIZATION,
        PROXY_AUTHORIZATION,
        COOKIE,
        SET_COOKIE,
        HeaderName::from_static("x-api-key"),
    ];

    pub fn new(headers: &HeaderMap, rules: &[Rule], cap: usize) -> Self {
        Self {
            request: Partial::new(headers, rules),
            response: None,
            cap,
        }
    }

    pub fn answer(&mut self, headers: &HeaderMap) {
        self.response = Some(Partial::new(headers, &[]));
    }

    pub fn push(&mut self, direction: Direction, chunk: &[u8]) {
        match direction {
            Direction::Sent => self.request.push(chunk, self.cap),
            Direction::Received => {
                if let Some(response) = &mut self.response {
                    response.push(chunk, self.cap);
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

    pub fn snapshot(&self, id: u64) -> Exchange {
        Exchange {
            id,
            request: self.request.snapshot(),
            response: self.response.as_ref().map(Partial::snapshot),
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

    fn push(&mut self, chunk: &[u8], cap: usize) {
        let kept = chunk.len().min(cap.saturating_sub(self.body.len()));
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

    fn snapshot(&self) -> Message {
        Message {
            headers: self.headers.clone(),
            body: Content::from_bytes(self.body.clone(), true),
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

    const CAP: usize = 64;

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
        let mut capture = Capture::new(&HeaderMap::new(), &[], CAP);
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
            CAP,
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
        let capture = Capture::new(
            &headers(&[("api-key", "real"), ("x-trace", "abc")]),
            &rules,
            CAP,
        );
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
        let mut capture = Capture::new(&HeaderMap::new(), &[], CAP);
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
        let filler = vec![b'a'; CAP - 1];
        let message = captured(&[&filler, "é".as_bytes(), b"rest"]);
        assert!(message.truncated);
        let Content::Text(text) = message.body else {
            panic!("a truncated text body became {:?}", message.body);
        };
        assert_eq!(text.len(), CAP - 1);
        assert!(text.bytes().all(|byte| byte == b'a'));
    }

    #[test]
    fn each_capture_stops_at_its_own_cap() {
        let mut capture = Capture::new(&HeaderMap::new(), &[], 4);
        capture.push(Direction::Sent, b"hello ");
        capture.push(Direction::Sent, b"world");
        capture.answer(&HeaderMap::new());
        capture.push(Direction::Received, b"ok");
        let exchange = capture.into_exchange(1);
        assert_eq!(exchange.request.body, Content::Text("hell".to_owned()));
        assert!(exchange.request.truncated);
        let response = exchange.response.unwrap();
        assert_eq!(response.body, Content::Text("ok".to_owned()));
        assert!(!response.truncated);
    }

    #[test]
    fn a_cap_of_zero_keeps_the_headers_and_no_body() {
        let mut capture = Capture::new(&headers(&[("x-trace", "abc")]), &[], 0);
        capture.push(Direction::Sent, b"hello");
        capture.answer(&headers(&[("content-type", "text/plain")]));
        capture.push(Direction::Received, b"world");
        let exchange = capture.into_exchange(1);
        assert_eq!(exchange.request.headers[0].0, "x-trace");
        assert_eq!(exchange.request.body, Content::Empty);
        assert!(exchange.request.truncated);
        let response = exchange.response.unwrap();
        assert_eq!(response.headers[0].0, "content-type");
        assert_eq!(response.body, Content::Empty);
        assert!(response.truncated);
    }

    #[test]
    fn an_empty_body_is_not_marked_as_cut_even_at_zero() {
        let mut capture = Capture::new(&HeaderMap::new(), &[], 0);
        capture.push(Direction::Sent, b"");
        assert!(!capture.into_exchange(1).request.truncated);
    }

    #[test]
    fn response_chunks_go_to_the_response_once_answered() {
        let mut capture = Capture::new(&HeaderMap::new(), &[], CAP);
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
    fn a_snapshot_shows_what_arrived_so_far_while_capturing_goes_on() {
        let mut capture = Capture::new(&headers(&[("authorization", "Bearer real")]), &[], CAP);
        capture.push(Direction::Sent, b"caf");
        capture.push(Direction::Sent, &"é".as_bytes()[..1]);
        let early = capture.snapshot(3);
        assert_eq!(early.id, 3);
        assert_eq!(early.request.headers[0].1, "<redacted>");
        assert_eq!(early.request.body, Content::Text("caf".to_owned()));
        assert!(early.response.is_none());
        capture.push(Direction::Sent, &"é".as_bytes()[1..]);
        capture.answer(&headers(&[("content-type", "text/plain")]));
        capture.push(Direction::Received, b"ok");
        let later = capture.snapshot(3);
        assert_eq!(later.request.body, Content::Text("café".to_owned()));
        assert_eq!(later.response.unwrap().body, Content::Text("ok".to_owned()));
        assert_eq!(capture.into_exchange(3).request.body, later.request.body);
    }

    #[test]
    fn debug_output_shows_sizes_not_contents() {
        let mut capture = Capture::new(&headers(&[("authorization", "Bearer real")]), &[], CAP);
        capture.push(Direction::Sent, b"secret body");
        let shown = format!("{capture:?}");
        assert!(shown.contains("headers: 1") && shown.contains("body: 11"));
        assert!(!shown.contains("real") && !shown.contains("secret"));
    }
}
