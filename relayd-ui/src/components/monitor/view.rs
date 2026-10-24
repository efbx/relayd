use dioxus::prelude::*;
use relayd_monitor::Entry;

use super::MonitorRow;
use crate::components::{Button, Window};
use crate::state::{AppState, Refresh};

#[component]
pub(super) fn MonitorView(
    number: Signal<usize>,
    version: Signal<u64>,
    on_page: EventHandler<usize>,
) -> Element {
    let state = AppState::current();
    let rows = use_store(Vec::<Entry>::new);
    let mut pages = use_signal(|| 1_usize);
    let open = use_signal(|| None::<u64>);
    use_resource(move || async move {
        _ = version();
        if let Ok(fresh) = relayd_monitor::entries(number()).await {
            let count = fresh.pages();
            if *pages.peek() != count {
                pages.set(count);
            }
            rows.refresh(fresh.items);
        }
    });
    let ids = use_memo(move || {
        rows.read()
            .iter()
            .map(|entry| entry.id)
            .collect::<Vec<u64>>()
    });
    let pages = pages();
    let number = number();

    rsx! {
        Window { title: "recent requests", tone: "orange",
            div { class: "monitor-bar",
                Button { ghost: true, disabled: number == 0, onclick: move |()| on_page.call(number.saturating_sub(1)), "‹" }
                span { class: "mono pager", "{number + 1} / {pages}" }
                Button { ghost: true, disabled: number + 1 >= pages, onclick: move |()| on_page.call(number + 1), "›" }
                span { class: "spacer" }
                Button { ghost: true, onclick: move |()| state.clear_monitor(), "clear" }
            }
            div { class: "table",
                div { class: "table-head",
                    span { "id" }
                    span { "when" }
                    span { "method" }
                    span { "path" }
                    span { "prefix" }
                    span { "stage" }
                    span { "status" }
                    span { "elapsed" }
                    span { "sent" }
                    span { "received" }
                }
                div { class: "table-body",
                    if ids.read().is_empty() {
                        p { class: "notice", "No requests yet." }
                    }
                    for (index, id) in ids.read().iter().enumerate() {
                        MonitorRow { key: "{id}", rows, index, id: *id, open, version }
                    }
                }
            }
        }
    }
}
