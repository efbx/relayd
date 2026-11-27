use dioxus::prelude::*;
use relayd_settings::BodyLimit;

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
    let units = use_memo(|| BodyLimit::UNITS.map(str::to_owned).to_vec());
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
                span { class: "setting-label", "history" }
                Field {
                    value: draft.history.clone(),
                    placeholder: "0",
                    mono: true,
                    variant: "grow",
                    onchange: move |history| state.set_history(history),
                }
                span { class: "setting-label", "prune every" }
                Field {
                    value: draft.history_prune_every.clone(),
                    placeholder: "1",
                    mono: true,
                    variant: "grow",
                    onchange: move |every| state.set_history_prune_every(every),
                }
            }
            p { class: "hint", "History: how many of the newest requests the monitor keeps; 0 stops recording new requests and their contents." }
            div { class: "setting",
                span { class: "setting-label", "stored contents" }
                Field {
                    value: draft.captures.clone(),
                    placeholder: "0",
                    mono: true,
                    variant: "grow",
                    onchange: move |captures| state.set_captures(captures),
                }
                span { class: "setting-label", "prune every" }
                Field {
                    value: draft.captures_prune_every.clone(),
                    placeholder: "1",
                    mono: true,
                    variant: "grow",
                    onchange: move |every| state.set_captures_prune_every(every),
                }
            }
            p { class: "hint", "Stored contents: how many of the newest requests keep their headers and bodies for the monitor; 0 stops storing new contents." }
            div { class: "setting",
                span { class: "setting-label", "body limit" }
                Field {
                    value: draft.captures_body.clone(),
                    placeholder: "0",
                    mono: true,
                    variant: "grow",
                    onchange: move |amount| state.set_captures_body(amount),
                }
                Select {
                    value: draft.captures_body_unit.clone(),
                    options: units,
                    onchange: move |unit| state.set_captures_body_unit(unit),
                }
            }
            p { class: "hint", "Body limit: how much of each request and response body is stored, decimals allowed; the rest streams through unrecorded and 0 stores the headers only. Applies to requests that start after it is saved." }
            p { class: "hint", "Prune every: how many requests a list may grow over its size before the oldest are dropped in one go. Both limits apply when the next request finishes; saving deletes nothing." }
            p { class: "hint", "Listen address applies after a restart" }
            div { class: "settings-actions",
                span { class: "spacer" }
                Button { onclick: move |()| state.restart_tunnel(), "Restart tunnel" }
                Button { ghost: true, onclick: move |()| state.restart_relayd(), "Restart RelayD" }
            }
        }
    }
}
