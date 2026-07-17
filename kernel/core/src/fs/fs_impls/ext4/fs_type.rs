// SPDX-License-Identifier: MPL-2.0

//! VFS filesystem-type registration for ext4.
//!
//! One driver registers both the `ext4` and compatibility `ext2` mount names.

use aster_systree::SysNode;
use device_id::DeviceId;

use super::{
    fs::{Ext4, MountFlavor},
    prelude::*,
};
use crate::fs::vfs::{
    file_system::FileSystem,
    registry::{FsCache, FsCreationCtx, FsProperties, FsType},
};

/// VFS-visible ext4 filesystem type.
pub(in crate::fs) struct Ext4Type {
    cache: FsCache<DeviceId>,
}

/// The VFS filesystem type descriptor for ext4 mounts.
pub(in crate::fs) static EXT4_TYPE: Ext4Type = Ext4Type {
    cache: FsCache::new(),
};

impl FsType for Ext4Type {
    type Key = DeviceId;

    fn name(&self) -> &'static str {
        "ext4"
    }

    fn properties(&self) -> FsProperties {
        FsProperties::NEED_DISK
    }

    fn create(&self, fs_creation_ctx: &mut FsCreationCtx) -> Result<Arc<dyn FileSystem>> {
        let disk = fs_creation_ctx.resolve_block_device()?.clone();
        let flags = fs_creation_ctx.flags();
        let args = fs_creation_ctx.args();
        Ext4::open(disk, flags, MountFlavor::Ext4, args).map(|fs| fs as Arc<dyn FileSystem>)
    }

    fn obtain_key_and_cache(
        &self,
        fs_creation_ctx: &mut FsCreationCtx,
    ) -> Option<(DeviceId, &FsCache<DeviceId>)> {
        let key = fs_creation_ctx
            .resolve_block_device()
            .ok()
            .map(|disk| disk.id())?;

        Some((key, &self.cache))
    }

    fn sysnode(&self) -> Option<Arc<dyn SysNode>> {
        None
    }
}

/// VFS-visible ext2 filesystem type, served by the unified driver.
pub(in crate::fs) struct Ext2Type {
    cache: FsCache<DeviceId>,
}

/// The VFS filesystem type descriptor for ext2 mounts.
pub(in crate::fs) static EXT2_TYPE: Ext2Type = Ext2Type {
    cache: FsCache::new(),
};

impl FsType for Ext2Type {
    type Key = DeviceId;

    fn name(&self) -> &'static str {
        "ext2"
    }

    fn properties(&self) -> FsProperties {
        FsProperties::NEED_DISK
    }

    fn create(&self, fs_creation_ctx: &mut FsCreationCtx) -> Result<Arc<dyn FileSystem>> {
        let disk = fs_creation_ctx.resolve_block_device()?.clone();
        let flags = fs_creation_ctx.flags();
        let args = fs_creation_ctx.args();
        Ext4::open(disk, flags, MountFlavor::Ext2, args).map(|fs| fs as Arc<dyn FileSystem>)
    }

    fn obtain_key_and_cache(
        &self,
        fs_creation_ctx: &mut FsCreationCtx,
    ) -> Option<(DeviceId, &FsCache<DeviceId>)> {
        let key = fs_creation_ctx
            .resolve_block_device()
            .ok()
            .map(|disk| disk.id())?;

        Some((key, &self.cache))
    }

    fn sysnode(&self) -> Option<Arc<dyn SysNode>> {
        None
    }
}
