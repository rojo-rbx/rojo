use memofs::Vfs;
use std::{
    path::{Path, PathBuf},
    rc::Rc,
};

use crate::snapshot::{InstanceWithMeta, RojoTree};
use rbx_dom_weak::{types::Ref, Instance, WeakDom};

pub struct SyncbackData<'new, 'old> {
    pub(super) vfs: &'old Vfs,
    pub(super) old_tree: &'old RojoTree,
    pub(super) new_tree: &'new WeakDom,
}

pub struct SyncbackSnapshot<'new, 'old> {
    pub(super) data: Rc<SyncbackData<'new, 'old>>,
    pub(super) old: Option<Ref>,
    pub(super) new: Ref,
    pub(super) parent_path: PathBuf,
    pub(super) name: String,
}

impl<'new, 'old> SyncbackSnapshot<'new, 'old> {
    /// Constructs a SyncbackSnapshot from the provided refs
    /// while inheriting the parent's trees and path
    #[inline]
    pub fn from_parent<P: AsRef<Path>>(
        &self,
        extension: P,
        new_name: String,
        new_ref: Ref,
        old_ref: Option<Ref>,
    ) -> Self {
        Self {
            data: Rc::clone(&self.data),
            old: old_ref,
            new: new_ref,
            parent_path: self.parent_path.join(extension.as_ref()),
            name: new_name,
        }
    }

    /// Returns an Instance from the old tree with the provided referent, if it
    /// exists.
    #[inline]
    pub fn get_old_instance(&self, referent: Ref) -> Option<InstanceWithMeta<'old>> {
        self.data.old_tree.get_instance(referent)
    }

    /// Returns an Instance from the new tree with the provided referent, if it
    /// exists.
    #[inline]
    pub fn get_new_instance(&self, referent: Ref) -> Option<&'new Instance> {
        self.data.new_tree.get_by_ref(referent)
    }

    /// The 'old' Instance this snapshot is for, if it exists.
    #[inline]
    pub fn old_inst(&self) -> Option<InstanceWithMeta<'old>> {
        self.old
            .and_then(|old| self.data.old_tree.get_instance(old))
    }

    /// The 'new' Instance this snapshot is for.
    #[inline]
    pub fn new_inst(&self) -> &'new Instance {
        self.data
            .new_tree
            .get_by_ref(self.new)
            .expect("SyncbackSnapshot should not contain invalid referents")
    }

    /// Returns the underlying VFS being used for syncback.
    #[inline]
    pub fn vfs(&self) -> &'old Vfs {
        self.data.vfs
    }

    /// Returns the WeakDom used for the 'new' tree
    #[inline]
    pub fn new_tree(&self) -> &'new WeakDom {
        self.data.new_tree
    }
}
