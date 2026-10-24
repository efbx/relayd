use dioxus::prelude::*;

use super::{RuleRow, TargetHeader};
use crate::components::{Button, Window};
use crate::state::{AppState, TargetDraftStoreExt};

#[component]
pub(crate) fn TargetCard(index: usize) -> Element {
    let state = AppState::current();
    let rules = state
        .targets
        .get(index)
        .map_or(0, |target| target.rules().len());

    rsx! {
        Window { title: "target {index + 1}", tone: "yellow",
            TargetHeader { index }
            div { class: "rules",
                for position in 0..rules {
                    RuleRow { key: "{position}", target: index, position }
                }
            }
            div { class: "target-foot",
                Button { ghost: true, onclick: move |()| state.add_rule(index), "+ rule" }
            }
        }
    }
}
