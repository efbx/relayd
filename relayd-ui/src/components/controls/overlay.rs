use dioxus::prelude::*;

use super::Window;

#[component]
pub(crate) fn Overlay(title: &'static str, children: Element) -> Element {
    rsx! {
        div { class: "overlay",
            Window { title: "RelayD", tone: "orange",
                div { class: "modal",
                    span { class: "spinner" }
                    span { class: "modal-title", {title} }
                    {children}
                }
            }
        }
    }
}
