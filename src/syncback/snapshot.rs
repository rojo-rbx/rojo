use memofs::Vfs;
use std::{
    collections::{BTreeSet, HashMap},
    path::PathBuf,
};

use crate::{
    glob::Glob,
    snapshot::{InstanceWithMeta, RojoTree},
    variant_eq::variant_eq,
};
use rbx_dom_weak::{
    types::{Ref, Variant},
    Instance, WeakDom,
};

use super::SyncbackRules;

#[derive(Clone, Copy)]
pub struct SyncbackData<'new, 'old> {
    pub(super) vfs: &'old Vfs,
    pub(super) old_tree: &'old RojoTree,
    pub(super) new_tree: &'new WeakDom,
    pub(super) syncback_rules: Option<&'old SyncbackRules>,
}

pub struct SyncbackSnapshot<'new, 'old> {
    pub data: SyncbackData<'new, 'old>,
    pub old: Option<Ref>,
    pub new: Ref,
    pub parent_path: PathBuf,
    pub name: String,
}

impl<'new, 'old> SyncbackSnapshot<'new, 'old> {
    /// Constructs a SyncbackSnapshot from the provided refs
    /// while inheriting the parent's trees and path
    #[inline]
    pub fn with_parent(&self, new_name: String, new_ref: Ref, old_ref: Option<Ref>) -> Self {
        Self {
            data: self.data,
            old: old_ref,
            new: new_ref,
            parent_path: self.parent_path.join(&self.name),
            name: new_name,
        }
    }

    /// Returns a map of properties for the 'new' Instance with filtering
    /// done to avoid noise.
    ///
    /// Note that while the returned map does filter based on the user's
    /// `ignore_props` field, it does not do any other filtering and doesn't
    /// clone any data. This is left to the consumer.
    #[inline]
    pub fn new_filtered_properties(&self) -> HashMap<&'new str, &'new Variant> {
        self.get_filtered_properties(self.new).unwrap()
    }

    /// Returns a map of properties for an Instance from the 'new' tree
    /// with filtering done to avoid noise.
    ///
    /// Note that while the returned map does filter based on the user's
    /// `ignore_props` field, it does not do any other filtering and doesn't
    /// clone any data. This is left to the consumer.
    pub fn get_filtered_properties(
        &self,
        new_ref: Ref,
    ) -> Option<HashMap<&'new str, &'new Variant>> {
        let inst = self.get_new_instance(new_ref)?;
        let mut properties: HashMap<&str, &Variant> =
            HashMap::with_capacity(inst.properties.capacity());

        let filter = self.get_property_filter();

        if let Some(old_inst) = self.old_inst() {
            for (name, value) in &inst.properties {
                if old_inst.properties().contains_key(name) {
                    properties.insert(name, value);
                }
            }
        } else {
            let class_data = rbx_reflection_database::get()
                .classes
                .get(inst.class.as_str());
            if let Some(class_data) = class_data {
                let defaults = &class_data.default_properties;
                for (name, value) in &inst.properties {
                    // We don't currently support refs or shared strings
                    if matches!(value, Variant::Ref(_) | Variant::SharedString(_)) {
                        continue;
                    }
                    if let Some(list) = &filter {
                        if list.contains(name) {
                            continue;
                        }
                    }
                    if let Some(default) = defaults.get(name.as_str()) {
                        if !variant_eq(value, default) {
                            properties.insert(name, value);
                        }
                    } else {
                        properties.insert(name, value);
                    }
                }
            } else {
                for (name, value) in &inst.properties {
                    // We don't currently support refs or shared strings
                    if matches!(value, Variant::Ref(_) | Variant::SharedString(_)) {
                        continue;
                    }
                    if let Some(list) = &filter {
                        if list.contains(name) {
                            continue;
                        }
                    }
                    properties.insert(name, value);
                }
            }
        }

        Some(properties)
    }

    /// Returns a set of properties that should not be written with syncback if
    /// one exists.
    fn get_property_filter(&self) -> Option<BTreeSet<&String>> {
        let filter = self.ignore_props()?;
        let mut set = BTreeSet::new();

        let database = rbx_reflection_database::get();
        let mut current_class_name = self.new_inst().class.as_str();

        loop {
            if let Some(list) = filter.get(current_class_name) {
                set.extend(list)
            }

            let class = database.classes.get(current_class_name)?;
            if let Some(super_class) = class.superclass.as_ref() {
                current_class_name = &super_class;
            } else {
                break;
            }
        }

        Some(set)
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

    /// Returns the WeakDom used for the 'new' tree.
    #[inline]
    pub fn new_tree(&self) -> &'new WeakDom {
        self.data.new_tree
    }

    /// Returns user-specified property ignore rules.
    #[inline]
    pub fn ignore_props(&self) -> Option<&HashMap<String, Vec<String>>> {
        self.data
            .syncback_rules
            .map(|rules| &rules.ignore_properties)
    }

    /// Returns user-specified ignore paths.
    #[inline]
    pub fn ignore_paths(&self) -> Option<&[Glob]> {
        self.data
            .syncback_rules
            .map(|rules| rules.ignore_paths.as_slice())
    }

    /// Returns user-specified ignore tree.
    #[inline]
    pub fn ignore_tree(&self) -> Option<&[String]> {
        self.data
            .syncback_rules
            .map(|rules| rules.ignore_trees.as_slice())
    }
}
