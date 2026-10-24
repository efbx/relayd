use dioxus::prelude::*;

use crate::components::{Field, IconButton, Select};
use crate::state::{AppState, TargetDraftStoreExt};

#[component]
pub(super) fn TargetHeader(index: usize) -> Element {
    let state = AppState::current();
    let Some(target) = state.targets.get(index) else {
        return rsx! {};
    };
    let vpn = target.vpn().cloned();
    let routed = !vpn.is_empty();

    rsx! {
        div { class: "target-head",
            Field {
                value: target.prefix().cloned(),
                placeholder: "/prefix",
                mono: true,
                variant: "prefix",
                onchange: move |prefix| state.set_prefix(index, prefix),
            }
            span { class: "arrow", "→" }
            Field {
                value: target.upstream().cloned(),
                placeholder: "https://upstream.example",
                mono: true,
                variant: "grow",
                onchange: move |upstream| state.set_upstream(index, upstream),
            }
            div { class: if routed { "vpn-pick" } else { "vpn-pick direct" },
                Select { value: vpn, none: "direct", options: state.vpn_names, onchange: move |name| state.set_target_vpn(index, name) }
                IconButton { label: "⟲", onclick: move |()| state.set_target_vpn(index, String::new()) }
            }
            IconButton { label: "✕", danger: true, onclick: move |()| state.remove_target(index) }
        }
    }
}
