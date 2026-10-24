use std::time::Duration;
#[cfg(feature = "standalone")]
use std::{
    env, io,
    process::{Child, Command},
};

use relayd_core::{DataDir, Error};
use relayd_monitor::{CaptureLimit, Monitor};
use relayd_proxy::{IdleTimeout, Listen};
use relayd_storage::FileStore;
use relayd_tunnel::{Tunnel, TunnelConfig};
use sylk::Pack;

use crate::Settings;

#[derive(Clone, Debug, Pack)]
pub struct SettingsStore {
    file: FileStore<Settings>,
    tunnel: Tunnel,
    idle_timeout: IdleTimeout,
    monitor: Monitor,
}

#[sylk::resource]
fn settings_file(dir: DataDir) -> Result<FileStore<Settings>, Error> {
    FileStore::open(dir.0.join("relayd.ron"))
}

#[sylk::resource]
fn listen(file: FileStore<Settings>) -> Listen {
    Listen(file.read().listen)
}

#[sylk::resource]
fn idle_timeout(file: FileStore<Settings>) -> IdleTimeout {
    IdleTimeout::new(file.read().idle_timeout)
}

#[sylk::resource]
fn capture_limit(file: FileStore<Settings>) -> CaptureLimit {
    CaptureLimit::new(file.read().captures)
}

#[sylk::resource]
fn tunnel_config(file: FileStore<Settings>) -> TunnelConfig {
    let settings = file.read();
    TunnelConfig {
        target: format!("http://{}", settings.listen),
        mode: settings.tunnel.clone(),
    }
}

impl SettingsStore {
    const RESTART_DELAY: Duration = Duration::from_millis(300);

    #[must_use]
    pub fn current(&self) -> Settings {
        self.file.read().clone()
    }

    pub fn save(&self, settings: Settings) -> Result<(), Error> {
        let mode = settings.tunnel.clone();
        let (idle_timeout, captures) = (settings.idle_timeout, settings.captures);
        self.file.set(settings)?;
        self.tunnel.reconfigure(mode);
        self.idle_timeout.set(idle_timeout);
        self.monitor.limit_captures(captures);
        Ok(())
    }

    pub fn restart(&self) {
        tokio::spawn(async {
            tokio::time::sleep(Self::RESTART_DELAY).await;
            #[cfg(feature = "standalone")]
            if Self::start_next_copy().is_err() {
                return;
            }
            std::process::exit(0);
        });
    }

    #[cfg(feature = "standalone")]
    fn start_next_copy() -> io::Result<Child> {
        Command::new(env::current_exe()?).spawn()
    }
}
