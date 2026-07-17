// SPDX-License-Identifier: MPL-2.0

//! Writeback and reclaim for ext4 inodes.

#![short_vis_path::add(ext4)]

use super::{
    super::{fs::Ext4, prelude::*, utils},
    Inode, InodeInner, InodePayload,
};

impl Inode {
    /// Flushes all dirty state and issues a device barrier.
    pub(in ext4) fn sync_all(&self) -> Result<()> {
        self.flush_xattr()?;
        let fs = self.fs()?;
        fs.sync_metadata()?;
        self.sync_data()
    }

    /// Flushes dirty file data and the metadata required to retrieve it, then
    /// issues a device barrier.
    pub(in ext4) fn sync_data(&self) -> Result<()> {
        self.sync_data_no_barrier()?;
        let fs = self.fs()?;
        if fs.block_device().sync()? != BioStatus::Complete {
            return_errno_with_message!(Errno::EIO, "failed to flush block device");
        }
        Ok(())
    }

    /// Flushes dirty data and inode metadata without a device barrier.
    fn sync_data_no_barrier(&self) -> Result<()> {
        let fs = self.fs()?;
        let mut inner = self.inner.write();
        inner.sync_data_pages()?;
        inner.write_back_inode_desc(&fs, self.ino)?;
        Ok(())
    }

    /// Flushes the inode's extended-attribute block, if present.
    fn flush_xattr(&self) -> Result<()> {
        match &self.xattr {
            Some(xattr) => xattr.flush(),
            None => Ok(()),
        }
    }

    /// Flushes all inode-owned state without issuing a device barrier.
    pub(in ext4) fn sync_all_no_barrier(&self) -> Result<()> {
        self.flush_xattr()?;
        self.sync_data_no_barrier()
    }

    /// Reclaims the storage owned by a fully unlinked inode.
    pub(super) fn try_reclaim_deleted_inode(&self) -> Result<bool> {
        if self.link_count() != 0 {
            return Ok(false);
        }

        let fs = self.fs()?;
        if !fs.is_inode_allocated(self.ino) {
            return Ok(false);
        }

        if let Some(xattr) = self.xattr.as_ref() {
            xattr.delete_xattr_block()?;
        }

        let mut inner = self.inner.write();
        let old_size = inner.file_size();
        // A fast symlink stores its target inline and owns no mapped data blocks.
        let block_manager = inner.block_manager().ok().cloned();
        if block_manager.is_some() {
            inner.resize_page_cache(0, old_size)?;
        }
        inner.set_dtime(utils::now());
        inner.set_file_size(0);
        inner.set_file_acl(0);
        // The mapping engine is authoritative until its accounting is written back.
        if let Some(block_manager) = block_manager
            && block_manager.sector_count() > 0
        {
            block_manager.truncate_to_byte_len(0)?;
        }
        inner.write_back_inode_desc(&fs, self.ino)?;

        fs.free_inode(self.ino, self.type_)?;
        Ok(true)
    }
}

impl InodeInner {
    /// Persists a dirty inode descriptor and clears its dirty state.
    pub(super) fn write_back_inode_desc(&mut self, fs: &Ext4, ino: Ext4Ino) -> Result<()> {
        if !self.is_dirty() {
            return Ok(());
        }
        // Engine-internal metadata blocks must be durable before the inode
        // that references them is written back.
        if let Ok(bm) = self.block_manager() {
            bm.sync_metadata()?;
        }
        let (root, sector_count) = match self.block_manager() {
            Ok(bm) => (bm.root_snapshot(), bm.sector_count()),
            Err(_) => (*self.desc.raw_block(), self.desc.sector_count()),
        };
        // Mirror the mapping engine's authoritative accounting before writeback.
        self.desc.set_sector_count(sector_count);
        fs.write_back_inode_desc(ino, &self.desc, &root)?;
        self.clear_dirty();
        Ok(())
    }

    /// Flushes dirty data pages in `[0, file_size)`.
    fn sync_data_pages(&self) -> Result<()> {
        let file_size = self.file_size();
        if file_size == 0 {
            return Ok(());
        }
        match &self.payload {
            InodePayload::DataBacked { page_cache, .. } => page_cache.flush_range(0..file_size),
            _ => Ok(()),
        }
    }
}
