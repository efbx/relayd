use dioxus::prelude::*;

use super::VpnRow;
use crate::components::Window;
use crate::state::AppState;

#[component]
pub(crate) fn VpnList() -> Element {
    let state = AppState::current();
    let count = state.vpns.len();

    rsx! {
        Window { title: "vpns", tone: "orange",
            if count == 0 {
                p { class: "notice", "No vpns yet. Paste a WireGuard config below." }
            }
            for index in 0..count {
                VpnRow { index }
            }
        }
    }
}
