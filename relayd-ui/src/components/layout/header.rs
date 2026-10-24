use dioxus::prelude::*;

use crate::components::{IconButton, Pill, Tab};
use crate::state::{AppState, Page};

#[component]
pub(crate) fn Header() -> Element {
    let state = AppState::current();
    use_future(move || state.watch_tunnel());
    let tunnel = (state.tunnel)();
    let live = !tunnel.is_empty();
    let on = (state.tunnel_on)();
    let current = (state.page)();
    let tone = if live { "live" } else { "starting" };

    rsx! {
        header { class: "top",
            span { class: "brand",
                span { class: "brand-mark" }
                "RelayD"
            }
            nav { class: "tabs",
                for page in Page::ALL {
                    Tab { label: page.label(), active: page == current, onclick: move |()| state.show(page) }
                }
            }
            span { class: "spacer" }
            if on {
                Pill { tone, dot: true,
                    span { class: "tunnel-url", if live { {tunnel} } else { "Starting…" } }
                    IconButton { label: "↺", onclick: move |()| state.restart_tunnel() }
                }
            } else {
                Pill { tone: "neutral", dot: true,
                    span { class: "tunnel-url", "No tunnel" }
                }
            }
        }
    }
}
