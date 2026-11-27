use dioxus::prelude::*;
use relayd_monitor::{Content, Exchange, Human, Stage};

#[derive(Clone, Copy, PartialEq)]
enum Part {
    Request,
    Response,
}

#[component]
pub(super) fn ExchangePanel(
    id: u64,
    stage: Stage,
    started: u64,
    elapsed: u64,
    sent: u64,
    received: u64,
) -> Element {
    let exchange = use_resource(use_reactive(
        (&stage, &sent, &received),
        move |_| async move { relayd_monitor::exchange(id).await.ok().flatten() },
    ));
    let loaded = exchange.read();
    let finished = stage.finished();

    rsx! {
        div { class: "table-detail",
            if finished {
                p { class: "hint", "Sent {Human::time(started)}" }
            } else {
                p { class: "hint", "Sent {Human::time(started)}, running for {Human::running(elapsed)}" }
            }
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
                    MessageView { exchange, part: Part::Request, stage, size: sent }
                    MessageView { exchange, part: Part::Response, stage, size: received }
                },
            }
        }
    }
}

#[component]
fn MessageView(
    exchange: Resource<Option<Exchange>>,
    part: Part,
    stage: Stage,
    size: u64,
) -> Element {
    let loaded = exchange.read();
    let Some(Some(exchange)) = loaded.as_ref() else {
        return rsx! {};
    };
    let (title, message, arriving) = match part {
        Part::Request => ("request", Some(&exchange.request), stage == Stage::Received),
        Part::Response => ("response", exchange.response.as_ref(), !stage.finished()),
    };

    rsx! {
        section { class: "exchange",
            h4 { class: "exchange-title", {title} }
            match message {
                None if arriving => rsx! {
                    p { class: "hint", "Waiting for the response…" }
                },
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
                    match &message.body {
                        Content::Empty if message.truncated => rsx! {
                            p { class: "hint", "Body not stored, {Human::size(size)}" }
                        },
                        Content::Empty if arriving => rsx! {},
                        Content::Empty => rsx! {
                            p { class: "hint", "No body" }
                        },
                        Content::Binary => rsx! {
                            p { class: "hint", "Binary body, {Human::size(size)}" }
                        },
                        Content::Text(text) => rsx! {
                            if message.truncated {
                                p { class: "hint", "First {Human::size(text.len() as u64)} of {Human::size(size)}" }
                            }
                            pre { class: "body", {text.as_str()} }
                        },
                    }
                    if arriving {
                        p { class: "hint", "More is still arriving…" }
                    }
                },
            }
        }
    }
}
