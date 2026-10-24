use dioxus::prelude::*;

#[component]
pub(crate) fn Select(
    value: String,
    none: &'static str,
    options: Memo<Vec<String>>,
    onchange: EventHandler<String>,
) -> Element {
    let mut open = use_signal(|| false);
    let missing = !value.is_empty() && !options.read().contains(&value);
    let shown = if value.is_empty() { none } else { &value };

    rsx! {
        div { class: if missing { "select select-missing" } else { "select" },
            button { r#type: "button", class: "select-trigger", onclick: move |_| open.toggle(),
                span { class: "select-value", {shown} }
                span { class: "select-caret", "▾" }
            }
            if open() {
                div { class: "select-scrim", onclick: move |_| open.set(false) }
                div { class: "select-popup",
                    button {
                        r#type: "button",
                        class: if value.is_empty() { "select-option selected" } else { "select-option" },
                        onclick: move |_| {
                            open.set(false);
                            onchange.call(String::new());
                        },
                        {none}
                    }
                    for name in options.read().iter().cloned() {
                        button {
                            key: "{name}",
                            r#type: "button",
                            class: if name == value { "select-option selected" } else { "select-option" },
                            onclick: move |_| {
                                open.set(false);
                                onchange.call(name.clone());
                            },
                            {name.clone()}
                        }
                    }
                    if missing {
                        span { class: "select-option missing", "{value} (missing)" }
                    }
                }
            }
        }
    }
}
