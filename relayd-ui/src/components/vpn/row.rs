use dioxus::prelude::*;

use crate::components::{IconButton, Pill};
use crate::state::AppState;

#[component]
pub(super) fn VpnRow(index: usize) -> Element {
    let state = AppState::current();
    let Some(vpn) = state.vpns.get(index) else {
        return rsx! {};
    };
    let vpn = vpn.read();
    let tone = if vpn.up { "live" } else { "starting" };

    rsx! {
        div { class: "vpn-row",
            span { class: "mono", "{vpn.name}" }
            Pill { tone, dot: true, if vpn.up { "up" } else { "starting…" } }
            IconButton { label: "✕", danger: true, onclick: move |()| state.remove_vpn(index) }
        }
    }
}
