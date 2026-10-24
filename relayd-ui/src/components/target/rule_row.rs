use dioxus::prelude::*;

use super::rule::Rule;
use crate::components::{Chip, Field, IconButton};
use crate::state::{AppState, TargetDraftStoreExt};

#[component]
pub(super) fn RuleRow(target: usize, position: usize) -> Element {
    let state = AppState::current();
    let Some(line) = state
        .targets
        .get(target)
        .and_then(|draft| draft.rules().get(position))
    else {
        return rsx! {};
    };
    let line = line.read();
    let rule = Rule::parse(&line);
    let verb = rule.verb;
    let kind = if rule.hidden() { "password" } else { "text" };
    let write = move |line: String| state.set_rule(target, position, line);

    rsx! {
        div { class: "rule",
            Chip { tone: verb.tone(), label: verb.label() }
            match (verb.takes_args(), rule.secret()) {
                (false, _) => rsx! { span { class: "rule-fill" } },
                (true, Some((name, value))) => {
                    let (name, value) = (name.to_owned(), value.to_owned());
                    let (kept_name, kept_value) = (name.clone(), value.clone());
                    rsx! {
                        Field {
                            value: name,
                            placeholder: "header",
                            mono: true,
                            variant: "name",
                            onchange: move |name: String| write(Rule::header(&name, &kept_value)),
                        }
                        Field {
                            value,
                            placeholder: "value",
                            kind: "password",
                            mono: true,
                            variant: "grow",
                            onchange: move |value: String| write(Rule::header(&kept_name, &value)),
                        }
                    }
                }
                (true, None) => rsx! {
                    Field {
                        value: rule.args.to_owned(),
                        placeholder: verb.hint(),
                        kind,
                        mono: true,
                        variant: "grow",
                        onchange: move |args: String| write(Rule::line(verb, args)),
                    }
                },
            }
            IconButton { label: "✕", danger: true, onclick: move |()| state.remove_rule(target, position) }
        }
    }
}
