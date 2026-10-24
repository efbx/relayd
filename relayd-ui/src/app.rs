use dioxus::prelude::*;

use crate::components::{Fonts, Header, Overlay};
use crate::pages::{MonitorPage, SettingsPage, TargetsPage, VpnPage};
use crate::state::{AppState, Page, Status};
#[cfg(feature = "standalone")]
use crate::title_bar::TitleBar;
#[cfg(not(feature = "standalone"))]
use crate::web_title_bar::TitleBar;

const MAIN_CSS: Asset = asset!("/assets/main.css");

#[component]
pub fn App() -> Element {
    let state = AppState::provide();

    rsx! {
        Fonts {}
        document::Stylesheet { href: MAIN_CSS }
        div { class: "app",
            TitleBar {}
            Header {}
            match (state.page)() {
                Page::Targets => rsx! { TargetsPage {} },
                Page::Monitor => rsx! { MonitorPage {} },
                Page::Vpn => rsx! { VpnPage {} },
                Page::Settings => rsx! { SettingsPage {} },
            }
            if *state.status.read() == Status::Restarting {
                Overlay { title: "Restarting…",
                    p { class: "muted", "RelayD is coming back on its own" }
                }
            }
        }
    }
}
