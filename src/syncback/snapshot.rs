use indexmap::IndexMap;
use memofs::Vfs;
use std::path::{Path, PathBuf};

use crate::{
    snapshot::{InstanceWithMeta, RojoTree},
    snapshot_middleware::Middleware,
    Project,
};
use rbx_dom_weak::{
    types::{Ref, Variant},
    Instance, Ustr, UstrMap, WeakDom,
};

use super::{get_best_middleware, name_for_inst, property_filter::filter_properties};

#[derive(Clone, Copy)]
pub struct SyncbackData<'sync> {
    pub(super) vfs: &'sync Vfs,
    pub(super) old_tree: &'sync RojoTree,
    pub(super) new_tree: &'sync WeakDom,
    pub(super) project: &'sync Project,
}

pub struct SyncbackSnapshot<'sync> {
    pub data: SyncbackData<'sync>,
    pub old: Option<Ref>,
    pub new: Ref,
    pub path: PathBuf,
    pub middleware: Option<Middleware>,
}

impl<'sync> SyncbackSnapshot<'sync> {
    /// Constructs a SyncbackSnapshot from the provided refs
    /// while inheriting this snapshot's path and data. This should be used for
    /// directories.
    #[inline]
    pub fn with_joined_path(&self, new_ref: Ref, old_ref: Option<Ref>) -> anyhow::Result<Self> {
        let mut snapshot = Self {
            data: self.data,
            old: old_ref,
            new: new_ref,
            path: PathBuf::new(),
            middleware: None,
        };
        let middleware = get_best_middleware(&snapshot);
        let name = name_for_inst(middleware, snapshot.new_inst(), snapshot.old_inst())?;
        snapshot.path = self.path.join(name.as_ref());

        Ok(snapshot)
    }

    /// Constructs a SyncbackSnapshot from the provided refs and a base path,
    /// while inheriting this snapshot's data.
    ///
    /// The actual path of the snapshot is made by getting a file name for the
    /// snapshot and then appending it to the provided base path.
    #[inline]
    pub fn with_base_path(
        &self,
        base_path: &Path,
        new_ref: Ref,
        old_ref: Option<Ref>,
    ) -> anyhow::Result<Self> {
        let mut snapshot = Self {
            data: self.data,
            old: old_ref,
            new: new_ref,
            path: PathBuf::new(),
            middleware: None,
        };
        let middleware = get_best_middleware(&snapshot);
        let name = name_for_inst(middleware, snapshot.new_inst(), snapshot.old_inst())?;
        snapshot.path = base_path.join(name.as_ref());

        Ok(snapshot)
    }

    /// Constructs a SyncbackSnapshot with the provided path and refs while
    /// inheriting the data of the this snapshot.
    #[inline]
    pub fn with_new_path(&self, path: PathBuf, new_ref: Ref, old_ref: Option<Ref>) -> Self {
        Self {
            data: self.data,
            old: old_ref,
            new: new_ref,
            path,
            middleware: None,
        }
    }

    /// Allows a middleware to be 'forced' onto a SyncbackSnapshot to override
    /// the attempts to derive it.
    #[inline]
    pub fn middleware(mut self, middleware: Middleware) -> Self {
        self.middleware = Some(middleware);
        self
    }

    /// Returns a map of properties for an Instance from the 'new' tree
    /// with filtering done to avoid noise. This method filters out properties
    /// that are not meant to be present in Instances that are represented
    /// specially by a path, like `LocalScript.Source` and `StringValue.Value`.
    ///
    /// This method is not necessary or desired for blobs like Rbxm or non-path
    /// middlewares like JsonModel.
    #[inline]
    #[must_use]
    pub fn get_path_filtered_properties(&self, new_ref: Ref) -> Option<UstrMap<&'sync Variant>> {
        let inst = self.get_new_instance(new_ref)?;

        // The only filtering we have to do is filter out properties that are
        // special-cased in some capacity.
        let properties = filter_properties(self.data.project, inst)
            .into_iter()
            .filter(|(name, _)| !filter_out_property(inst, name))
            .collect();

        Some(properties)
    }

    /// Returns a path to the provided Instance in the new DOM. This path is
    /// where you would look for the object in Roblox Studio.
    #[inline]
    pub fn get_new_inst_path(&self, referent: Ref) -> String {
        inst_path(self.new_tree(), referent)
    }

    /// Returns a path to the provided Instance in the old DOM. This path is
    /// where you would look for the object in Roblox Studio.
    #[inline]
    pub fn get_old_inst_path(&self, referent: Ref) -> String {
        inst_path(self.old_tree(), referent)
    }

    /// Returns an Instance from the old tree with the provided referent, if it
    /// exists.
    #[inline]
    pub fn get_old_instance(&self, referent: Ref) -> Option<InstanceWithMeta<'sync>> {
        self.data.old_tree.get_instance(referent)
    }

    /// Returns an Instance from the new tree with the provided referent, if it
    /// exists.
    #[inline]
    pub fn get_new_instance(&self, referent: Ref) -> Option<&'sync Instance> {
        self.data.new_tree.get_by_ref(referent)
    }

    /// The 'old' Instance this snapshot is for, if it exists.
    #[inline]
    pub fn old_inst(&self) -> Option<InstanceWithMeta<'sync>> {
        self.old
            .and_then(|old| self.data.old_tree.get_instance(old))
    }

    /// The 'new' Instance this snapshot is for.
    #[inline]
    pub fn new_inst(&self) -> &'sync Instance {
        self.data
            .new_tree
            .get_by_ref(self.new)
            .expect("SyncbackSnapshot should not contain invalid referents")
    }

    /// Returns the root Project that was used to make this snapshot.
    #[inline]
    pub fn project(&self) -> &'sync Project {
        self.data.project
    }

    /// Returns the underlying VFS being used for syncback.
    #[inline]
    pub fn vfs(&self) -> &'sync Vfs {
        self.data.vfs
    }

    /// Returns the WeakDom used for the 'new' tree.
    #[inline]
    pub fn new_tree(&self) -> &'sync WeakDom {
        self.data.new_tree
    }

    /// Returns the WeakDom used for the 'old' tree.
    #[inline]
    pub fn old_tree(&self) -> &'sync WeakDom {
        self.data.old_tree.inner()
    }

    /// Returns user-specified property ignore rules.
    #[inline]
    pub fn ignore_props(&self) -> Option<&IndexMap<Ustr, Vec<Ustr>>> {
        self.data
            .project
            .syncback_rules
            .as_ref()
            .map(|rules| &rules.ignore_properties)
    }

    /// Returns user-specified ignore tree.
    #[inline]
    pub fn ignore_tree(&self) -> Option<&[String]> {
        self.data
            .project
            .syncback_rules
            .as_ref()
            .map(|rules| rules.ignore_trees.as_slice())
    }
}

pub fn filter_out_property(inst: &Instance, prop_name: &str) -> bool {
    match inst.class.as_str() {
        "Script" | "LocalScript" | "ModuleScript" => {
            // These properties shouldn't be set by scripts that are created via
            // `$path` or via being on the file system.
            prop_name == "Source" || prop_name == "ScriptGuid"
        }
        "LocalizationTable" => prop_name == "Contents",
        "StringValue" => prop_name == "Value",
        _ => false,
    }
}

pub fn inst_path(dom: &WeakDom, referent: Ref) -> String {
    let mut path = Vec::new();

    let mut inst = dom.get_by_ref(referent);
    while let Some(instance) = inst {
        path.push(instance.name.as_str());
        inst = dom.get_by_ref(instance.parent());
    }
    // This is to avoid the root's name from appearing in the path. Not
    // optimal, but should be fine.
    path.pop();

    path.reverse();
    path.join("/")
}

#[cfg(test)]
mod test {
    use rbx_dom_weak::{InstanceBuilder, WeakDom};

    use super::inst_path as inst_path_outer;

    #[test]
    fn inst_path() {
        let mut new_tree = WeakDom::new(InstanceBuilder::new("ROOT"));

        let child_1 = new_tree.insert(new_tree.root_ref(), InstanceBuilder::new("Child1"));
        let child_2 = new_tree.insert(child_1, InstanceBuilder::new("Child2"));
        let child_3 = new_tree.insert(child_2, InstanceBuilder::new("Child3"));

        assert_eq!(inst_path_outer(&new_tree, new_tree.root_ref()), "");
        assert_eq!(inst_path_outer(&new_tree, child_1), "Child1");
        assert_eq!(inst_path_outer(&new_tree, child_2), "Child1/Child2");
        assert_eq!(inst_path_outer(&new_tree, child_3), "Child1/Child2/Child3");
    }
}
