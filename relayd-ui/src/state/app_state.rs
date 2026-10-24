use std::{sync::Arc, time::Duration};

use dioxus::core::spawn_forever;
use dioxus::prelude::*;
use dioxus::stores::index::IndexWrite;
use dioxus_fullstack::ServerFnError;
use dioxus_sdk_time::sleep;
use futures_util::StreamExt;
use relayd_monitor::clear_history;
use relayd_settings::{restart, save_settings, settings};
use relayd_targets::TargetList;
use relayd_targets::{save_targets, targets};
use relayd_tunnel::{restart_tunnel, tunnel, tunnel_enabled, watch_tunnel};
use relayd_vpn::{VpnInfo, add_vpn, remove_vpn, vpns};

use super::{Page, Refresh, SettingsDraft, Status, TargetDraft, TargetDraftStoreExt};

type DraftStore = Store<TargetDraft, IndexWrite<usize, WriteSignal<Vec<TargetDraft>>>>;

#[derive(Clone, Copy)]
pub(crate) struct AppState {
    pub(crate) page: Signal<Page>,
    pub(crate) targets: Store<Vec<TargetDraft>>,
    saved_targets: Signal<Vec<TargetDraft>>,
    pub(crate) settings: Signal<Option<SettingsDraft>>,
    saved_settings: Signal<Option<SettingsDraft>>,
    pub(crate) tunnel: Signal<String>,
    pub(crate) tunnel_on: Signal<bool>,
    pub(crate) vpns: Store<Vec<VpnInfo>>,
    pub(crate) vpn_names: Memo<Vec<String>>,
    pub(crate) vpn_name: Signal<String>,
    pub(crate) vpn_config: Signal<String>,
    pub(crate) status: Signal<Status>,
}

impl AppState {
    const RETRY: Duration = Duration::from_secs(2);

    pub(crate) fn provide() -> Self {
        let vpns = use_store(Vec::<VpnInfo>::new);
        let state = Self {
            page: use_signal(|| Page::Targets),
            targets: use_store(Vec::new),
            saved_targets: use_signal(Vec::new),
            settings: use_signal(|| None),
            saved_settings: use_signal(|| None),
            tunnel: use_signal(String::new),
            tunnel_on: use_signal(|| true),
            vpns,
            vpn_names: use_memo(move || vpns.read().iter().map(|vpn| vpn.name.clone()).collect()),
            vpn_name: use_signal(String::new),
            vpn_config: use_signal(String::new),
            status: use_signal(|| Status::Loading),
        };
        use_hook(move || {
            state.refresh_tunnel();
            state.load();
        });
        use_context_provider(|| state)
    }

    pub(crate) fn current() -> Self {
        use_context()
    }

    pub(crate) fn show(mut self, page: Page) {
        self.page.set(page);
    }

    pub(crate) fn targets_dirty(self) -> bool {
        *self.targets.read() != *self.saved_targets.read()
    }

    pub(crate) fn settings_dirty(self) -> bool {
        *self.settings.read() != *self.saved_settings.read()
    }

    pub(crate) fn discard_targets(mut self) {
        self.targets.set(self.saved_targets.cloned());
        self.status.set(Status::Ready);
    }

    pub(crate) fn discard_settings(mut self) {
        self.settings.set(self.saved_settings.cloned());
        self.status.set(Status::Ready);
    }

    pub(crate) fn refresh_tunnel(mut self) {
        spawn(async move {
            self.tunnel_on.set(tunnel_enabled().await.unwrap_or(true));
            self.tunnel.set(tunnel().await.unwrap_or_default());
        });
    }

    pub(crate) fn restart_tunnel(mut self) {
        spawn_forever(async move {
            if let Err(error) = restart_tunnel().await {
                self.fail(error);
                return;
            }
            self.tunnel.set(String::new());
        });
    }

    pub(crate) async fn watch_tunnel(mut self) {
        loop {
            if let Ok(stream) = watch_tunnel().await {
                let mut stream = stream.into_inner();
                while let Some(Ok(url)) = stream.next().await {
                    self.tunnel.set(url);
                }
            }
            sleep(Self::RETRY).await;
            self.tunnel.set(tunnel().await.unwrap_or_default());
        }
    }

    pub(crate) fn restart_relayd(mut self) {
        spawn_forever(async move {
            let _ = restart().await;
            self.status.set(Status::Restarting);
            self.tunnel.set(String::new());
            loop {
                sleep(Self::RETRY).await;
                if let Ok(loaded) = targets().await {
                    self.apply_targets(loaded);
                    self.refresh_tunnel();
                    break;
                }
            }
        });
    }

    fn load(self) {
        spawn(async move {
            match targets().await {
                Ok(loaded) => self.apply_targets(loaded),
                Err(error) => self.fail(error),
            }
        });
    }

    fn apply_targets(mut self, loaded: TargetList) {
        let drafts: Vec<TargetDraft> = loaded
            .targets
            .into_iter()
            .map(Arc::unwrap_or_clone)
            .map(TargetDraft::from_target)
            .collect();
        self.saved_targets.set(drafts.clone());
        self.targets.set(drafts);
        self.status.set(Status::Ready);
    }

    fn fail(mut self, error: ServerFnError) {
        self.status.set(Status::Failed(Self::message(error)));
    }

    fn mark_edited(mut self) {
        if *self.status.peek() != Status::Edited {
            self.status.set(Status::Edited);
        }
    }

    pub(crate) fn clear_monitor(self) {
        spawn(async move {
            if let Err(error) = clear_history().await {
                self.fail(error);
            }
        });
    }

    pub(crate) fn load_settings(mut self) {
        spawn(async move {
            match settings().await {
                Ok(loaded) => {
                    let draft = Some(SettingsDraft::from_settings(loaded));
                    self.saved_settings.set(draft.clone());
                    self.settings.set(draft);
                }
                Err(error) => self.fail(error),
            }
        });
    }

    fn edit_settings(mut self, edit: impl FnOnce(&mut SettingsDraft)) {
        if let Some(draft) = self.settings.write().as_mut() {
            edit(draft);
        }
        self.mark_edited();
    }

    pub(crate) fn set_listen(self, listen: String) {
        self.edit_settings(|draft| draft.listen = listen);
    }

    pub(crate) fn set_tunnel_mode(self, tunnel: String) {
        self.edit_settings(|draft| draft.tunnel = tunnel);
    }

    pub(crate) fn set_tunnel_token(self, token: String) {
        self.edit_settings(|draft| draft.token = token);
    }

    pub(crate) fn set_idle_timeout(self, idle_timeout: String) {
        self.edit_settings(|draft| draft.idle_timeout = idle_timeout);
    }

    pub(crate) fn set_captures(self, captures: String) {
        self.edit_settings(|draft| draft.captures = captures);
    }

    pub(crate) fn save_settings(mut self) {
        let parsed = match self.settings.read().as_ref().map(SettingsDraft::parse) {
            Some(Ok(parsed)) => parsed,
            Some(Err(message)) => {
                self.status.set(Status::Failed(message));
                return;
            }
            None => return,
        };
        self.status.set(Status::Saving);
        spawn(async move {
            match save_settings(parsed).await {
                Ok(()) => {
                    self.saved_settings.set(self.settings.cloned());
                    self.status.set(Status::Saved);
                }
                Err(error) => self.fail(error),
            }
        });
    }

    pub(crate) fn save_targets(mut self) {
        let mut parsed = Vec::new();
        for draft in self.targets.read().iter() {
            match draft.parse() {
                Ok(target) => parsed.push(Arc::new(target)),
                Err(message) => {
                    self.status.set(Status::Failed(message));
                    return;
                }
            }
        }
        self.status.set(Status::Saving);
        spawn(async move {
            match save_targets(TargetList { targets: parsed }).await {
                Ok(()) => {
                    self.saved_targets.set(self.targets.cloned());
                    self.status.set(Status::Saved);
                }
                Err(error) => self.fail(error),
            }
        });
    }

    fn message(error: ServerFnError) -> String {
        match error {
            ServerFnError::ServerError { message, .. } => message,
            other => other.to_string(),
        }
    }

    fn edit(self, index: usize, edit: impl FnOnce(DraftStore)) {
        if let Some(target) = self.targets.get(index) {
            edit(target);
        }
        self.mark_edited();
    }

    pub(crate) fn add_target(mut self) {
        self.targets.push(TargetDraft::default());
        self.mark_edited();
    }

    pub(crate) fn remove_target(mut self, index: usize) {
        if index < self.targets.len() {
            self.targets.remove(index);
        }
        self.mark_edited();
    }

    pub(crate) fn set_prefix(self, index: usize, prefix: String) {
        self.edit(index, |target| target.prefix().set(prefix));
    }

    pub(crate) fn set_upstream(self, index: usize, upstream: String) {
        self.edit(index, |target| target.upstream().set(upstream));
    }

    pub(crate) fn add_rule(self, index: usize) {
        self.edit(index, |target| target.rules().push(String::new()));
    }

    pub(crate) fn set_rule(self, index: usize, position: usize, line: String) {
        self.edit(index, |target| {
            if let Some(mut rule) = target.rules().get(position) {
                rule.set(line);
            }
        });
    }

    pub(crate) fn remove_rule(self, index: usize, position: usize) {
        self.edit(index, |target| {
            let mut rules = target.rules();
            if position < rules.len() {
                rules.remove(position);
            }
        });
    }

    pub(crate) fn set_target_vpn(self, index: usize, vpn: String) {
        self.edit(index, |target| target.vpn().set(vpn));
    }

    pub(crate) fn load_vpns(self) {
        spawn(async move {
            match vpns().await {
                Ok(loaded) => self.vpns.refresh(loaded),
                Err(error) => self.fail(error),
            }
        });
    }

    pub(crate) fn set_vpn_name(mut self, name: String) {
        self.vpn_name.set(name);
    }

    pub(crate) fn set_vpn_config(mut self, config: String) {
        self.vpn_config.set(config);
    }

    pub(crate) fn vpn_form_filled(self) -> bool {
        !self.vpn_name.read().trim().is_empty() && !self.vpn_config.read().trim().is_empty()
    }

    pub(crate) fn add_vpn(mut self) {
        if !self.vpn_form_filled() {
            self.status.set(Status::Failed(
                "vpn: name and config are required".to_owned(),
            ));
            return;
        }
        let (name, config) = (
            self.vpn_name.read().trim().to_owned(),
            self.vpn_config.cloned(),
        );
        self.status.set(Status::Saving);
        spawn(async move {
            match add_vpn(name, config).await {
                Ok(()) => {
                    self.vpn_name.set(String::new());
                    self.vpn_config.set(String::new());
                    self.status.set(Status::Saved);
                    self.load_vpns();
                }
                Err(error) => self.fail(error),
            }
        });
    }

    pub(crate) fn remove_vpn(self, index: usize) {
        let Some(name) = self.vpns.get(index).map(|vpn| vpn.read().name.clone()) else {
            return;
        };
        spawn(async move {
            match remove_vpn(name).await {
                Ok(()) => self.load_vpns(),
                Err(error) => self.fail(error),
            }
        });
    }
}
