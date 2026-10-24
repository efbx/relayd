use std::{
    collections::BTreeMap,
    sync::{
        Arc, Mutex, MutexGuard,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

use axum::http::{HeaderMap, header::CONTENT_LENGTH};
use futures_util::{Stream, stream};
use relayd_core::{Body, Ctx, DataDir, Error, Incoming, Interceptor, Outgoing, Reply, Rule};
use relayd_storage::{Page, RdbStore};
use sylk::Pack;
use tokio::time::MissedTickBehavior;

use crate::{ByteMeter, Capture, CaptureLimit, Direction, Entry, Event, Exchange};

#[derive(Clone, Debug, Pack)]
pub struct Monitor {
    store: RdbStore<Entry>,
    exchanges: RdbStore<Exchange>,
    captures: CaptureLimit,
    #[pack(func = Arc::default())]
    live: Arc<Mutex<BTreeMap<u64, Entry>>>,
    #[pack(func = Arc::default())]
    pending: Arc<Mutex<BTreeMap<u64, Capture>>>,
    #[pack(func = Arc::default())]
    version: Arc<AtomicU64>,
}

#[sylk::resource]
fn history(dir: DataDir) -> Result<RdbStore<Entry>, Error> {
    RdbStore::open(dir.0.join("relayd.redb"))
}

#[sylk::resource]
fn exchanges(history: RdbStore<Entry>) -> RdbStore<Exchange> {
    history.sibling()
}

impl Monitor {
    const PAGE: usize = 20;
    const KEEP: u64 = 10_000;
    const PRUNE_EVERY: u64 = 256;
    const TICK: Duration = Duration::from_millis(500);

    pub(crate) fn record(&self, entry: Entry) {
        self.persist(&entry);
        self.live().insert(entry.id, entry);
        self.changed();
    }

    pub(crate) fn update(&self, id: u64, change: impl FnOnce(&mut Entry)) {
        let mut live = self.live();
        let Some(entry) = live.get_mut(&id) else {
            return;
        };
        let stage = entry.stage;
        change(entry);
        let finished = entry.finished();
        let changed = if finished {
            live.remove(&id)
        } else if entry.stage != stage {
            Some(entry.clone())
        } else {
            None
        };
        drop(live);
        if let Some(changed) = changed {
            self.persist(&changed);
            if finished {
                tracing::info!("{changed}");
                self.trim(changed.id);
                self.store_capture(changed.id);
            }
        }
        self.changed();
    }

    pub fn page(&self, number: usize) -> Result<Page<Entry>, Error> {
        let mut page = self.store.page(number, Self::PAGE)?;
        let live = self.live();
        for row in &mut page.items {
            if let Some(fresh) = live.get(&row.id) {
                *row = fresh.clone();
            }
        }
        Ok(page)
    }

    pub fn exchange(&self, id: u64) -> Result<Option<Exchange>, Error> {
        self.exchanges.get(id)
    }

    pub fn limit_captures(&self, count: u64) {
        self.captures.set(count);
        if let Err(error) = self.forget_captures_beyond(count) {
            tracing::error!("history: {}", error.message);
        }
        self.changed();
    }

    pub fn clear(&self) -> Result<(), Error> {
        self.live().clear();
        self.pending().clear();
        self.store.clear()?;
        self.exchanges.clear()?;
        self.changed();
        Ok(())
    }

    pub fn changes(&self) -> impl Stream<Item = u64> + Send + 'static {
        let mut ticker = tokio::time::interval(Self::TICK);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
        stream::unfold(
            (self.clone(), ticker, None),
            |(monitor, mut ticker, mut seen)| async move {
                loop {
                    ticker.tick().await;
                    let version = monitor.version.load(Ordering::Relaxed);
                    if seen == Some(version) {
                        continue;
                    }
                    seen = Some(version);
                    return Some((version, (monitor, ticker, seen)));
                }
            },
        )
    }

    fn changed(&self) {
        self.version.fetch_add(1, Ordering::Relaxed);
    }

    fn persist(&self, entry: &Entry) {
        if let Err(error) = self.store.put(entry) {
            tracing::error!("history: {}", error.message);
        }
    }

    fn trim(&self, newest: u64) {
        if !newest.is_multiple_of(Self::PRUNE_EVERY) {
            return;
        }
        match self.store.prune(newest.saturating_sub(Self::KEEP)) {
            Ok(0) => {}
            Ok(_) => self.changed(),
            Err(error) => tracing::error!("history: {}", error.message),
        }
    }

    fn start_capture(&self, id: u64, headers: &HeaderMap, rules: &[Rule]) {
        let mut pending = self.pending();
        if self.captures.count() > 0 {
            pending.insert(id, Capture::new(headers, rules));
        }
    }

    fn answer_capture(&self, id: u64, headers: &HeaderMap) {
        if let Some(capture) = self.pending().get_mut(&id) {
            capture.answer(headers);
        }
    }

    pub(crate) fn capture(&self, id: u64, direction: Direction, chunk: &[u8]) {
        if let Some(capture) = self.pending().get_mut(&id) {
            capture.push(direction, chunk);
        }
    }

    fn is_capturing(&self, id: u64) -> bool {
        self.pending().contains_key(&id)
    }

    fn store_capture(&self, id: u64) {
        let Some(capture) = self.pending().remove(&id) else {
            return;
        };
        let count = self.captures.count();
        if count == 0 {
            return;
        }
        let exchange = capture.into_exchange(id);
        if let Err(error) = self.exchanges.put_keeping(&exchange, count) {
            tracing::error!("history: {}", error.message);
        }
    }

    fn forget_captures_beyond(&self, count: u64) -> Result<(), Error> {
        if count == 0 {
            self.pending().clear();
            return self.exchanges.clear();
        }
        self.exchanges.keep_newest(count).map(drop)
    }

    fn live(&self) -> MutexGuard<'_, BTreeMap<u64, Entry>> {
        self.live
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn pending(&self) -> MutexGuard<'_, BTreeMap<u64, Capture>> {
        self.pending
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

#[sylk::register(order = 20)]
impl Interceptor for Monitor {
    fn on_request(&self, ctx: &Ctx, outgoing: &mut Outgoing) -> Result<(), Box<Reply>> {
        self.record(Entry::new(ctx));
        self.start_capture(ctx.id, &outgoing.headers, &ctx.target.rules);
        Ok(())
    }

    fn on_response(&self, ctx: &Ctx, incoming: &mut Incoming) -> Result<(), Box<Reply>> {
        self.answer_capture(ctx.id, &incoming.headers);
        let status = incoming.status.as_u16();
        let expected = incoming
            .headers
            .get(CONTENT_LENGTH)
            .and_then(|value| value.to_str().ok()?.parse().ok());
        self.update(ctx.id, |entry| {
            entry.apply(Event::Answered(status, expected));
        });
        Ok(())
    }

    fn tap_request(&self, ctx: &Ctx, body: Body) -> Body {
        let capturing = self.is_capturing(ctx.id);
        ByteMeter::wrap(body, self.clone(), ctx.id, Direction::Sent, capturing)
    }

    fn tap_response(&self, ctx: &Ctx, body: Body) -> Body {
        let capturing = self.is_capturing(ctx.id);
        ByteMeter::wrap(body, self.clone(), ctx.id, Direction::Received, capturing)
    }

    fn on_cancel(&self, ctx: &Ctx) {
        self.update(ctx.id, |entry| entry.apply(Event::Cancelled));
    }
}

#[cfg(test)]
mod tests {
    use std::{borrow::Cow, io};

    use axum::{
        body::Bytes,
        http::{HeaderName, HeaderValue},
    };
    use futures_util::StreamExt;

    use super::*;
    use crate::{Content, Stage};

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

    fn monitor(name: &str) -> Monitor {
        let path =
            std::env::temp_dir().join(format!("relayd-monitor-{name}-{}.redb", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let store = RdbStore::open(path).unwrap();
        Monitor {
            exchanges: store.sibling(),
            store,
            captures: CaptureLimit::new(2),
            live: Arc::default(),
            pending: Arc::default(),
            version: Arc::default(),
        }
    }

    fn entry(id: u64) -> Entry {
        Entry {
            id,
            method: "GET".into(),
            path: "/x".into(),
            prefix: "/".into(),
            stage: Stage::Received,
            status: None,
            started: 0,
            elapsed: 0,
            sent: 0,
            received: 0,
            expected: None,
        }
    }

    fn ids(page: &Page<Entry>) -> Vec<u64> {
        page.items.iter().map(|entry| entry.id).collect()
    }

    #[test]
    fn pages_hold_twenty_newest_first() {
        let monitor = monitor("pages");
        for id in 1..=25 {
            monitor.record(entry(id));
        }
        let first = monitor.page(0).unwrap();
        assert_eq!(
            (
                first.total,
                first.pages(),
                first.items.len(),
                first.items[0].id
            ),
            (25, 2, 20, 25)
        );
        assert_eq!(ids(&monitor.page(1).unwrap()), [5, 4, 3, 2, 1]);
    }

    #[test]
    fn pages_show_live_byte_counts_before_they_are_stored() {
        let monitor = monitor("live");
        monitor.record(entry(1));
        monitor.update(1, |entry| entry.sent = 9);
        assert_eq!(monitor.page(0).unwrap().items[0].sent, 9);
        assert_eq!(monitor.store.get(1).unwrap().unwrap().sent, 0);
    }

    #[test]
    fn finishing_stores_the_final_counts_and_leaves_memory() {
        let monitor = monitor("finish");
        monitor.record(entry(1));
        monitor.update(1, |entry| entry.sent = 9);
        monitor.update(1, |entry| entry.stage = Stage::Done);
        assert!(monitor.live().is_empty());
        let stored = monitor.store.get(1).unwrap().unwrap();
        assert!(stored.stage == Stage::Done && stored.sent == 9);
    }

    #[test]
    fn finished_entries_stop_taking_updates() {
        let monitor = monitor("finished");
        monitor.record(entry(1));
        monitor.update(1, |entry| entry.stage = Stage::Failed);
        monitor.update(1, |entry| entry.stage = Stage::Done);
        assert_eq!(monitor.store.get(1).unwrap().unwrap().stage, Stage::Failed);
    }

    #[test]
    fn clear_empties_everything() {
        let monitor = monitor("clear");
        monitor.record(entry(1));
        monitor.clear().unwrap();
        assert_eq!(monitor.page(0).unwrap().total, 0);
    }

    #[test]
    fn every_change_moves_the_version() {
        let monitor = monitor("version");
        let start = monitor.version.load(Ordering::Relaxed);
        monitor.record(entry(1));
        monitor.update(1, |entry| entry.sent = 1);
        monitor.clear().unwrap();
        assert_eq!(monitor.version.load(Ordering::Relaxed), start + 3);
    }

    fn answered_reply(monitor: &Monitor, chunks: Vec<Result<Bytes, io::Error>>) -> Body {
        reply_of(monitor, 200, None, chunks)
    }

    fn reply_of(
        monitor: &Monitor,
        status: u16,
        expected: Option<u64>,
        chunks: Vec<Result<Bytes, io::Error>>,
    ) -> Body {
        monitor.record(entry(1));
        monitor.update(1, |entry| entry.apply(Event::Answered(status, expected)));
        let body = Body::from_stream(stream::iter(chunks));
        ByteMeter::wrap(body, monitor.clone(), 1, Direction::Received, false)
    }

    async fn exchanged(monitor: &Monitor, id: u64, body: &'static [u8]) {
        monitor.record(entry(id));
        let sent = headers(&[
            ("content-type", "application/json"),
            ("authorization", "Bearer x"),
        ]);
        monitor.start_capture(id, &sent, &[]);
        monitor.answer_capture(id, &HeaderMap::new());
        monitor.update(id, |entry| entry.apply(Event::Answered(200, None)));
        let chunks = [Ok::<_, io::Error>(Bytes::from_static(body))];
        let body = Body::from_stream(stream::iter(chunks));
        let mut data = ByteMeter::wrap(body, monitor.clone(), id, Direction::Received, true)
            .into_data_stream();
        while data.next().await.is_some() {}
    }

    #[tokio::test]
    async fn finished_requests_keep_their_contents_up_to_the_limit() {
        let monitor = monitor("captures");
        for id in 1..=3 {
            exchanged(&monitor, id, b"body").await;
        }
        assert!(monitor.exchange(1).unwrap().is_none());
        let third = monitor.exchange(3).unwrap().unwrap();
        assert_eq!(
            third.request.headers,
            [
                ("content-type".to_owned(), Cow::Borrowed("application/json")),
                ("authorization".to_owned(), Cow::Borrowed("<redacted>")),
            ]
        );
        assert_eq!(
            third.response.unwrap().body,
            Content::Text("body".to_owned())
        );
        assert!(monitor.pending().is_empty());
    }

    #[test]
    fn turning_capture_off_mid_flight_stores_nothing() {
        let monitor = monitor("captures-midflight");
        monitor.record(entry(1));
        monitor.start_capture(1, &HeaderMap::new(), &[]);
        monitor.captures.set(0);
        monitor.update(1, |entry| entry.apply(Event::Cancelled));
        assert!(monitor.exchange(1).unwrap().is_none());
        assert!(monitor.pending().is_empty());
    }

    #[tokio::test]
    async fn a_reply_of_known_size_keeps_its_whole_body() {
        let monitor = monitor("captures-sized");
        monitor.record(entry(1));
        monitor.start_capture(1, &HeaderMap::new(), &[]);
        monitor.answer_capture(1, &HeaderMap::new());
        monitor.update(1, |entry| entry.apply(Event::Answered(200, Some(4))));
        let mut data = ByteMeter::wrap(
            Body::from_stream(stream::iter(two_chunks())),
            monitor.clone(),
            1,
            Direction::Received,
            true,
        )
        .into_data_stream();
        while data.next().await.is_some() {}
        let exchange = monitor.exchange(1).unwrap().unwrap();
        assert_eq!(
            exchange.response.unwrap().body,
            Content::Text("abcd".to_owned())
        );
    }

    #[tokio::test]
    async fn nothing_is_captured_when_the_limit_is_zero() {
        let monitor = monitor("captures-off");
        monitor.captures.set(0);
        exchanged(&monitor, 1, b"body").await;
        assert!(monitor.exchange(1).unwrap().is_none());
        assert!(monitor.pending().is_empty());
    }

    #[tokio::test]
    async fn lowering_the_limit_drops_older_contents_at_once() {
        let monitor = monitor("captures-lower");
        monitor.captures.set(5);
        for id in 1..=3 {
            exchanged(&monitor, id, b"x").await;
        }
        monitor.limit_captures(1);
        assert!(monitor.exchange(2).unwrap().is_none());
        assert!(monitor.exchange(3).unwrap().is_some());
        monitor.limit_captures(0);
        assert!(monitor.exchange(3).unwrap().is_none());
    }

    #[test]
    fn a_late_finisher_older_than_the_newest_is_not_kept() {
        let monitor = monitor("captures-late");
        monitor.captures.set(1);
        for id in 1..=3 {
            monitor.record(entry(id));
            monitor.start_capture(id, &HeaderMap::new(), &[]);
        }
        monitor.update(3, |entry| entry.apply(Event::Cancelled));
        monitor.update(1, |entry| entry.apply(Event::Cancelled));
        assert!(monitor.exchange(1).unwrap().is_none());
        assert!(monitor.exchange(3).unwrap().is_some());
        assert_eq!(monitor.exchanges.count().unwrap(), 1);
    }

    #[test]
    fn a_cancelled_request_keeps_what_it_had() {
        let monitor = monitor("captures-cancel");
        monitor.record(entry(1));
        monitor.start_capture(1, &HeaderMap::new(), &[]);
        monitor.capture(1, Direction::Sent, b"partial");
        monitor.update(1, |entry| entry.apply(Event::Cancelled));
        let exchange = monitor.exchange(1).unwrap().unwrap();
        assert_eq!(exchange.request.body, Content::Text("partial".to_owned()));
        assert!(exchange.response.is_none());
    }

    #[test]
    fn clear_empties_contents_too() {
        let monitor = monitor("captures-clear");
        monitor.record(entry(1));
        monitor.start_capture(1, &HeaderMap::new(), &[]);
        monitor.update(1, |entry| entry.apply(Event::Cancelled));
        monitor.clear().unwrap();
        assert!(monitor.exchange(1).unwrap().is_none());
    }

    fn two_chunks() -> Vec<Result<Bytes, io::Error>> {
        vec![Ok(Bytes::from_static(b"ab")), Ok(Bytes::from_static(b"cd"))]
    }

    #[tokio::test]
    async fn a_reply_read_to_the_end_is_done() {
        let monitor = monitor("reply-done");
        let mut data = answered_reply(&monitor, two_chunks()).into_data_stream();
        while data.next().await.is_some() {}
        let stored = monitor.store.get(1).unwrap().unwrap();
        assert!(stored.stage == Stage::Done && stored.received == 4);
    }

    #[tokio::test]
    async fn a_reply_dropped_halfway_is_cancelled() {
        let monitor = monitor("reply-dropped");
        let mut data = answered_reply(&monitor, two_chunks()).into_data_stream();
        data.next().await;
        drop(data);
        let stored = monitor.store.get(1).unwrap().unwrap();
        assert!(stored.stage == Stage::Cancelled && stored.received == 2);
        assert!(monitor.live().is_empty());
    }

    #[tokio::test]
    async fn a_sized_error_dropped_after_its_last_byte_is_failed() {
        let monitor = monitor("reply-sized");
        let chunks = vec![Ok(Bytes::from_static(
            b"{\"error\":\"Internal Server Error\"}",
        ))];
        let mut data = reply_of(&monitor, 500, Some(33), chunks).into_data_stream();
        data.next().await;
        drop(data);
        let stored = monitor.store.get(1).unwrap().unwrap();
        assert!(stored.stage == Stage::Failed && stored.received == 33);
        assert!(monitor.live().is_empty());
    }

    #[tokio::test]
    async fn a_reply_that_breaks_upstream_is_failed() {
        let monitor = monitor("reply-broken");
        let chunks = vec![
            Ok(Bytes::from_static(b"ab")),
            Err(io::Error::other("upstream reset")),
        ];
        let mut data = answered_reply(&monitor, chunks).into_data_stream();
        while data.next().await.is_some() {}
        drop(data);
        assert_eq!(monitor.store.get(1).unwrap().unwrap().stage, Stage::Failed);
    }

    #[test]
    fn a_cancelled_request_leaves_memory() {
        let monitor = monitor("cancelled");
        monitor.record(entry(1));
        monitor.update(1, |entry| entry.apply(Event::Cancelled));
        assert!(monitor.live().is_empty());
        assert_eq!(
            monitor.store.get(1).unwrap().unwrap().stage,
            Stage::Cancelled
        );
    }

    #[tokio::test(start_paused = true)]
    async fn changes_sends_the_current_version_then_only_new_ones() {
        let monitor = monitor("changes");
        let mut changes = Box::pin(monitor.changes());
        let first = changes.next().await.unwrap();
        monitor.record(entry(1));
        let second = changes.next().await.unwrap();
        assert!(second > first);
        let quiet = tokio::time::timeout(Duration::from_secs(5), changes.next()).await;
        assert!(quiet.is_err());
    }
}
