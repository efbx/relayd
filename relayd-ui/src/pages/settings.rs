use dioxus::prelude::*;

use crate::components::{Bar, Button, SettingsForm};
use crate::state::{AppState, Status};

#[component]
pub(crate) fn SettingsPage() -> Element {
    let state = AppState::current();
    use_hook(move || state.load_settings());
    let dirty = use_memo(move || state.settings_dirty());
    let saving = *state.status.read() == Status::Saving;

    rsx! {
        main { class: "content",
            SettingsForm {}
        }
        Bar {
            end: rsx! {
                Button { ghost: true, disabled: !dirty(), onclick: move |()| state.discard_settings(), "Discard" }
                Button { primary: true, disabled: !dirty() || saving, onclick: move |()| state.save_settings(), "Save" }
            },
        }
    }
}
