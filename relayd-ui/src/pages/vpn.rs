use std::time::Duration;

use dioxus::prelude::*;
use dioxus_sdk_time::use_interval;

use crate::components::{Bar, Button, VpnForm, VpnList};
use crate::state::{AppState, Status};

#[component]
pub(crate) fn VpnPage() -> Element {
    let state = AppState::current();
    use_hook(move || state.load_vpns());
    use_interval(Duration::from_secs(3), move |()| state.load_vpns());
    let filled = use_memo(move || state.vpn_form_filled());
    let saving = *state.status.read() == Status::Saving;

    rsx! {
        main { class: "content",
            VpnList {}
            VpnForm {}
        }
        Bar {
            end: rsx! {
                Button { primary: true, disabled: !filled() || saving, onclick: move |()| state.add_vpn(), "Add vpn" }
            },
        }
    }
}
