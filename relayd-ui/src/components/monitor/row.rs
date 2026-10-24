use dioxus::prelude::*;
use relayd_monitor::{Entry, Human, Stage};

use super::ExchangePanel;
use crate::components::Chip;

#[component]
pub(super) fn MonitorRow(
    rows: Store<Vec<Entry>>,
    index: usize,
    id: u64,
    mut open: Signal<Option<u64>>,
    version: Signal<u64>,
) -> Element {
    let is_open = use_memo(move || open() == Some(id));
    let Some(entry) = rows.get(index) else {
        return rsx! {};
    };
    let entry = entry.read();
    let (tone, label) = match entry.stage {
        Stage::Received => ("neutral", "RECEIVED"),
        Stage::Forwarded => ("neutral", "FORWARDED"),
        Stage::Answered => ("info", "ANSWERED"),
        Stage::Done => ("ok", "DONE"),
        Stage::Failed => ("bad", "FAILED"),
        Stage::Cancelled => ("neutral", "CANCELLED"),
    };
    let expanded = is_open();

    rsx! {
        div {
            class: if expanded { "table-row open" } else { "table-row" },
            onclick: move |_| open.set(if *is_open.peek() { None } else { Some(id) }),
            span { class: "num", "{entry.id}" }
            span { class: "num", title: "{Human::time(entry.started)}", "{Human::clock(entry.started)}" }
            span { class: "method", "{entry.method}" }
            span { class: "path", "{entry.path}" }
            span { class: "mono", "{entry.prefix}" }
            Chip { tone, label }
            span { class: "num", "{entry.status_label()}" }
            span { class: "num", "{entry.elapsed_label()}" }
            span { class: "num", "{Human::size(entry.sent)}" }
            span { class: "num", "{Human::size(entry.received)}" }
        }
        if expanded {
            ExchangePanel {
                id,
                version,
                started: entry.started,
                finished: entry.finished(),
                sent: entry.sent,
                received: entry.received,
            }
        }
    }
}
