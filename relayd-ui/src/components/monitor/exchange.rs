use dioxus::prelude::*;
use relayd_monitor::{Content, Exchange, Human};

#[derive(Clone, Copy, PartialEq)]
enum Part {
    Request,
    Response,
}

#[component]
pub(super) fn ExchangePanel(
    id: u64,
    version: Signal<u64>,
    started: u64,
    finished: bool,
    sent: u64,
    received: u64,
) -> Element {
    let mut stored = use_signal(|| false);
    let exchange = use_resource(move || async move {
        if !finished && !*stored.peek() {
            _ = version();
        }
        let fetched = relayd_monitor::exchange(id).await.ok().flatten();
        if fetched.is_some() {
            stored.set(true);
        }
        fetched
    });
    let loaded = exchange.read();

    rsx! {
        div { class: "table-detail",
            p { class: "hint", "Sent {Human::time(started)}" }
            match loaded.as_ref() {
                None => rsx! {
                    p { class: "hint", "Loading…" }
                },
                Some(None) if !finished => rsx! {
                    p { class: "hint", "Still running…" }
                },
                Some(None) => rsx! {
                    p { class: "hint", "No stored contents: this request is older than the stored ones, or stored contents are off." }
                },
                Some(Some(_)) => rsx! {
                    MessageView { exchange, part: Part::Request, size: sent }
                    MessageView { exchange, part: Part::Response, size: received }
                },
            }
        }
    }
}

#[component]
fn MessageView(exchange: Resource<Option<Exchange>>, part: Part, size: u64) -> Element {
    let loaded = exchange.read();
    let Some(Some(exchange)) = loaded.as_ref() else {
        return rsx! {};
    };
    let (title, message) = match part {
        Part::Request => ("request", Some(&exchange.request)),
        Part::Response => ("response", exchange.response.as_ref()),
    };

    rsx! {
        section { class: "exchange",
            h4 { class: "exchange-title", {title} }
            match message {
                None => rsx! {
                    p { class: "hint", "No response was received." }
                },
                Some(message) => rsx! {
                    div { class: "headers",
                        for (name, value) in &message.headers {
                            span { class: "header-name", {name.as_str()} }
                            span { class: "header-value", {value.as_ref()} }
                        }
                    }
                    if message.truncated {
                        p { class: "hint", "First {Human::size(Exchange::BODY_CAP as u64)} of {Human::size(size)}" }
                    }
                    match &message.body {
                        Content::Empty => rsx! {
                            p { class: "hint", "No body" }
                        },
                        Content::Binary => rsx! {
                            p { class: "hint", "Binary body, {Human::size(size)}" }
                        },
                        Content::Text(text) => rsx! {
                            pre { class: "body", {text.as_str()} }
                        },
                    }
                },
            }
        }
    }
}
