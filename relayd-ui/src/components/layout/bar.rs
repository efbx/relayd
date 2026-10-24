use dioxus::prelude::*;

use crate::components::Pill;
use crate::state::AppState;

#[component]
pub(crate) fn Bar(start: Option<Element>, end: Option<Element>) -> Element {
    let state = AppState::current();
    let status = state.status.read();

    rsx! {
        footer { class: "bar",
            div { class: "bar-start", {start} }
            Pill { tone: status.tone(), dot: true, "{status.label()}" }
            div { class: "bar-end", {end} }
        }
    }
}
