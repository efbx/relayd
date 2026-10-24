use dioxus::prelude::*;

use crate::components::{Bar, Button, TargetCard};
use crate::state::{AppState, Status};

#[component]
pub(crate) fn TargetsPage() -> Element {
    let state = AppState::current();
    use_hook(move || state.load_vpns());
    let dirty = use_memo(move || state.targets_dirty());
    let count = state.targets.len();
    let empty = count == 0 && *state.status.read() != Status::Loading;
    let saving = *state.status.read() == Status::Saving;

    rsx! {
        main { class: "content",
            for index in 0..count {
                TargetCard { key: "{index}", index }
            }
            if empty {
                p { class: "notice", "No targets yet. Add one with + target." }
            }
        }
        Bar {
            start: rsx! { Button { onclick: move |()| state.add_target(), "+ target" } },
            end: rsx! {
                Button { ghost: true, disabled: !dirty(), onclick: move |()| state.discard_targets(), "Discard" }
                Button { primary: true, disabled: !dirty() || saving, onclick: move |()| state.save_targets(), "Save" }
            },
        }
    }
}
