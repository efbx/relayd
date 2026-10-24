use dioxus::prelude::*;

#[component]
pub(crate) fn Chip(tone: &'static str, label: &'static str) -> Element {
    rsx! {
        span { class: "chip chip-{tone}", {label} }
    }
}
