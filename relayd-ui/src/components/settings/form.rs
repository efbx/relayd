use dioxus::prelude::*;

use crate::components::{Button, Field, Select, Window};
use crate::state::{AppState, SettingsDraft};

#[component]
pub(crate) fn SettingsForm() -> Element {
    let state = AppState::current();
    let modes = use_memo(|| {
        vec![
            SettingsDraft::QUICK.to_owned(),
            SettingsDraft::DOMAIN.to_owned(),
        ]
    });
    let settings = state.settings.read();
    let Some(draft) = settings.as_ref() else {
        return rsx! {
            p { class: "notice", "Loading settings…" }
        };
    };

    rsx! {
        Window { title: "settings", tone: "amber",
            div { class: "setting",
                span { class: "setting-label", "listen" }
                Field {
                    value: draft.listen.clone(),
                    placeholder: "127.0.0.1:8585",
                    mono: true,
                    variant: "grow",
                    onchange: move |listen| state.set_listen(listen),
                }
            }
            div { class: "setting",
                span { class: "setting-label", "tunnel" }
                Select {
                    value: draft.tunnel.clone(),
                    none: SettingsDraft::OFF,
                    options: modes,
                    onchange: move |mode| state.set_tunnel_mode(mode),
                }
            }
            if draft.wants_token() {
                div { class: "setting",
                    span { class: "setting-label", "token" }
                    Field {
                        value: draft.token.clone(),
                        placeholder: "tunnel token from the Cloudflare dashboard",
                        kind: "password",
                        mono: true,
                        variant: "grow",
                        onchange: move |token| state.set_tunnel_token(token),
                    }
                }
                p { class: "hint", "In Cloudflare, point the tunnel's public hostname at http://{draft.listen}" }
            }
            div { class: "setting",
                span { class: "setting-label", "idle timeout" }
                Field {
                    value: draft.idle_timeout.clone(),
                    placeholder: "0",
                    mono: true,
                    variant: "grow",
                    onchange: move |idle_timeout| state.set_idle_timeout(idle_timeout),
                }
            }
            p { class: "hint", "Idle timeout: seconds an upstream may stay silent before RelayD gives up; every packet restarts the count, 0 is off. Applies to new requests as soon as it is saved." }
            div { class: "setting",
                span { class: "setting-label", "stored contents" }
                Field {
                    value: draft.captures.clone(),
                    placeholder: "0",
                    mono: true,
                    variant: "grow",
                    onchange: move |captures| state.set_captures(captures),
                }
            }
            p { class: "hint", "Stored contents: how many of the newest requests keep their headers and bodies for the monitor, up to 1 MiB per body; older ones are dropped, 0 is off. Applies as soon as it is saved." }
            p { class: "hint", "Listen address applies after a restart" }
            div { class: "settings-actions",
                span { class: "spacer" }
                Button { onclick: move |()| state.restart_tunnel(), "Restart tunnel" }
                Button { ghost: true, onclick: move |()| state.restart_relayd(), "Restart RelayD" }
            }
        }
    }
}
