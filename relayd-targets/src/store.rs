use std::sync::RwLockReadGuard;

use relayd_core::{DataDir, Error};
use relayd_storage::FileStore;
use sylk::Pack;

use crate::TargetList;

#[derive(Clone, Debug, Pack)]
pub struct TargetStore {
    file: FileStore<TargetList>,
}

#[sylk::resource]
fn targets_file(dir: DataDir) -> Result<FileStore<TargetList>, Error> {
    FileStore::open(dir.0.join("targets.ron"))
}

impl TargetStore {
    pub fn read(&self) -> RwLockReadGuard<'_, TargetList> {
        self.file.read()
    }

    pub fn save(&self, targets: TargetList) -> Result<(), Error> {
        self.file.set(targets)
    }
}
