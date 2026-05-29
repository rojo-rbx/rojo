//! Shared logic for merging a Rojo project tree into a base Roblox file.
//!
//! Used by both `rojo build --base` and `rojo upload --base`.

use std::{io::BufReader, path::Path};

use anyhow::{bail, Context};
use fs_err::File;
use rbx_dom_weak::{types::Ref, Instance, WeakDom};

use crate::snapshot::RojoTree;

/// A pair of (base instance, Rojo instance) whose children still need merging.
struct MergePair {
    base_ref: Ref,
    rojo_ref: Ref,
}

/// Reads a base Roblox file (.rbxl, .rbxlx, .rbxm, .rbxmx) into a WeakDom.
pub fn read_base_dom(path: &Path) -> anyhow::Result<WeakDom> {
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .context("Base file must have a .rbxl, .rbxlx, .rbxm, or .rbxmx extension")?;

    let content = BufReader::new(
        File::open(path)
            .with_context(|| format!("Could not open base file at {}", path.display()))?,
    );

    match extension {
        "rbxl" | "rbxm" => rbx_binary::from_reader(content)
            .with_context(|| format!("Could not deserialize binary file at {}", path.display())),
        "rbxlx" | "rbxmx" => {
            let config = rbx_xml::DecodeOptions::new()
                .property_behavior(rbx_xml::DecodePropertyBehavior::ReadUnknown);
            rbx_xml::from_reader(content, config)
                .with_context(|| format!("Could not deserialize XML file at {}", path.display()))
        }
        _ => bail!(
            "Base file must be .rbxl, .rbxlx, .rbxm, or .rbxmx, got .{}",
            extension
        ),
    }
}

/// Merges the Rojo project tree into a base WeakDom.
///
/// For place projects (root class is "DataModel"), iteratively merges Rojo's
/// children into matching base children by name and className. For model
/// projects, clones the entire Rojo tree as a child of the base root.
///
/// The `ignore_unknown_instances` metadata on each Rojo instance controls
/// whether unmatched base children are preserved or deleted:
/// - `true` (default for project nodes without `$path`): preserve
/// - `false` (default when `$path` is set): delete
pub fn merge_rojo_into_base(mut base: WeakDom, rojo: &RojoTree) -> anyhow::Result<WeakDom> {
    let rojo_root_id = rojo.get_root_id();
    let rojo_inner = rojo.inner();
    let rojo_root = rojo_inner
        .get_by_ref(rojo_root_id)
        .expect("Rojo tree should have a root");
    let base_root_ref = base.root_ref();

    // For model projects (root is not DataModel), clone entire Rojo tree
    // as a child of the base root.
    if rojo_root.class.as_str() != "DataModel" {
        let cloned_ref = rojo_inner.clone_into_external(rojo_root_id, &mut base);
        base.transfer_within(cloned_ref, base_root_ref);
        return Ok(base);
    }

    // For place projects: iteratively merge children using a stack.
    let mut stack = vec![MergePair {
        base_ref: base_root_ref,
        rojo_ref: rojo_root_id,
    }];

    while let Some(pair) = stack.pop() {
        let rojo_children: Vec<Ref> = rojo_inner
            .get_by_ref(pair.rojo_ref)
            .expect("Rojo instance should exist")
            .children()
            .to_vec();

        let base_children: Vec<Ref> = base
            .get_by_ref(pair.base_ref)
            .expect("Base instance should exist")
            .children()
            .to_vec();

        let mut base_matched = vec![false; base_children.len()];

        for &rojo_child_ref in &rojo_children {
            let rojo_child = rojo_inner
                .get_by_ref(rojo_child_ref)
                .expect("Rojo child should exist");

            // Find the first unmatched base child with the same name+class.
            let base_match = base_children
                .iter()
                .enumerate()
                .find(|(idx, base_child_ref)| {
                    if base_matched[*idx] {
                        return false;
                    }
                    let base_child = base
                        .get_by_ref(**base_child_ref)
                        .expect("Base child should exist");
                    base_child.name == rojo_child.name
                        && base_child.class.as_str() == rojo_child.class.as_str()
                });

            match base_match {
                Some((idx, &base_child_ref)) => {
                    base_matched[idx] = true;
                    update_properties(&mut base, base_child_ref, rojo_child);
                    stack.push(MergePair {
                        base_ref: base_child_ref,
                        rojo_ref: rojo_child_ref,
                    });
                }
                None => {
                    let cloned_ref = rojo_inner.clone_into_external(rojo_child_ref, &mut base);
                    base.transfer_within(cloned_ref, pair.base_ref);
                }
            }
        }

        // Delete unmatched base children if ignore_unknown_instances is false.
        let ignore_unknown = rojo
            .get_metadata(pair.rojo_ref)
            .map(|m| m.ignore_unknown_instances)
            .unwrap_or(true);

        if !ignore_unknown {
            for (idx, &base_child_ref) in base_children.iter().enumerate() {
                if !base_matched[idx] {
                    base.destroy(base_child_ref);
                }
            }
        }
    }

    Ok(base)
}

/// Overlays Rojo properties onto a base instance. Properties that exist only
/// in the base instance are preserved; properties from Rojo overwrite any
/// existing base values.
fn update_properties(base: &mut WeakDom, base_ref: Ref, rojo_instance: &Instance) {
    let base_instance = base
        .get_by_ref_mut(base_ref)
        .expect("Base instance should exist");

    base_instance.name.clone_from(&rojo_instance.name);

    for (prop_name, prop_value) in &rojo_instance.properties {
        base_instance
            .properties
            .insert(*prop_name, prop_value.clone());
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::snapshot::{InstanceMetadata, InstanceSnapshot, RojoTree};
    use rbx_dom_weak::{types::Variant, ustr, InstanceBuilder};

    #[test]
    fn merge_adds_new_children() {
        let mut base = WeakDom::new(InstanceBuilder::new("DataModel"));
        base.insert(
            base.root_ref(),
            InstanceBuilder::new("ReplicatedStorage").with_name("ReplicatedStorage"),
        );

        let rojo = RojoTree::new(
            InstanceSnapshot::new()
                .name("Game")
                .class_name("DataModel")
                .metadata(InstanceMetadata::new().ignore_unknown_instances(true))
                .children(vec![InstanceSnapshot::new()
                    .name("ReplicatedStorage")
                    .class_name("ReplicatedStorage")
                    .metadata(InstanceMetadata::new().ignore_unknown_instances(true))
                    .children(vec![InstanceSnapshot::new()
                        .name("NewScript")
                        .class_name("ModuleScript")])]),
        );

        let merged = merge_rojo_into_base(base, &rojo).unwrap();

        let root_children = merged.root().children();
        assert_eq!(root_children.len(), 1);

        let rs = merged.get_by_ref(root_children[0]).unwrap();
        assert_eq!(rs.name, "ReplicatedStorage");
        assert_eq!(rs.children().len(), 1);

        let child = merged.get_by_ref(rs.children()[0]).unwrap();
        assert_eq!(child.name, "NewScript");
        assert_eq!(child.class.as_str(), "ModuleScript");
    }

    #[test]
    fn merge_deletes_unknown_when_not_ignored() {
        let mut base = WeakDom::new(InstanceBuilder::new("DataModel"));
        let rs_ref = base.insert(
            base.root_ref(),
            InstanceBuilder::new("ReplicatedStorage").with_name("ReplicatedStorage"),
        );
        base.insert(
            rs_ref,
            InstanceBuilder::new("ModuleScript").with_name("OldScript"),
        );

        let rojo = RojoTree::new(
            InstanceSnapshot::new()
                .name("Game")
                .class_name("DataModel")
                .metadata(InstanceMetadata::new().ignore_unknown_instances(true))
                .children(vec![InstanceSnapshot::new()
                    .name("ReplicatedStorage")
                    .class_name("ReplicatedStorage")
                    .metadata(InstanceMetadata::new().ignore_unknown_instances(false))
                    .children(vec![InstanceSnapshot::new()
                        .name("NewScript")
                        .class_name("ModuleScript")])]),
        );

        let merged = merge_rojo_into_base(base, &rojo).unwrap();

        let root_children = merged.root().children();
        let rs = merged.get_by_ref(root_children[0]).unwrap();
        assert_eq!(rs.children().len(), 1, "OldScript should have been deleted");

        let child = merged.get_by_ref(rs.children()[0]).unwrap();
        assert_eq!(child.name, "NewScript");
    }

    #[test]
    fn merge_preserves_unknown_when_ignored() {
        let mut base = WeakDom::new(InstanceBuilder::new("DataModel"));
        let rs_ref = base.insert(
            base.root_ref(),
            InstanceBuilder::new("ReplicatedStorage").with_name("ReplicatedStorage"),
        );
        base.insert(
            rs_ref,
            InstanceBuilder::new("ModuleScript").with_name("OldScript"),
        );

        let rojo = RojoTree::new(
            InstanceSnapshot::new()
                .name("Game")
                .class_name("DataModel")
                .metadata(InstanceMetadata::new().ignore_unknown_instances(true))
                .children(vec![InstanceSnapshot::new()
                    .name("ReplicatedStorage")
                    .class_name("ReplicatedStorage")
                    .metadata(InstanceMetadata::new().ignore_unknown_instances(true))
                    .children(vec![InstanceSnapshot::new()
                        .name("NewScript")
                        .class_name("ModuleScript")])]),
        );

        let merged = merge_rojo_into_base(base, &rojo).unwrap();

        let root_children = merged.root().children();
        let rs = merged.get_by_ref(root_children[0]).unwrap();
        assert_eq!(
            rs.children().len(),
            2,
            "Both OldScript and NewScript should be present"
        );
    }

    #[test]
    fn merge_preserves_unmanaged_services() {
        let mut base = WeakDom::new(InstanceBuilder::new("DataModel"));
        base.insert(
            base.root_ref(),
            InstanceBuilder::new("ReplicatedStorage").with_name("ReplicatedStorage"),
        );
        base.insert(
            base.root_ref(),
            InstanceBuilder::new("Workspace").with_name("Workspace"),
        );

        // Rojo project only manages ReplicatedStorage, not Workspace
        let rojo = RojoTree::new(
            InstanceSnapshot::new()
                .name("Game")
                .class_name("DataModel")
                .metadata(InstanceMetadata::new().ignore_unknown_instances(true))
                .children(vec![InstanceSnapshot::new()
                    .name("ReplicatedStorage")
                    .class_name("ReplicatedStorage")
                    .metadata(InstanceMetadata::new().ignore_unknown_instances(true))]),
        );

        let merged = merge_rojo_into_base(base, &rojo).unwrap();

        let root_children = merged.root().children();
        assert_eq!(
            root_children.len(),
            2,
            "Both ReplicatedStorage and Workspace should be present"
        );
    }

    #[test]
    fn merge_overlays_properties() {
        let mut base = WeakDom::new(InstanceBuilder::new("DataModel"));
        base.insert(
            base.root_ref(),
            InstanceBuilder::new("ModuleScript")
                .with_name("Script")
                .with_property("Source", Variant::String("-- base source".into()))
                .with_property("Disabled", Variant::Bool(true)),
        );

        let rojo = RojoTree::new(
            InstanceSnapshot::new()
                .name("Game")
                .class_name("DataModel")
                .metadata(InstanceMetadata::new().ignore_unknown_instances(true))
                .children(vec![InstanceSnapshot::new()
                    .name("Script")
                    .class_name("ModuleScript")
                    .property(ustr("Source"), Variant::String("-- rojo source".into()))]),
        );

        let merged = merge_rojo_into_base(base, &rojo).unwrap();

        let script = merged.get_by_ref(merged.root().children()[0]).unwrap();

        // Rojo property overwrites base
        assert_eq!(
            script.properties.get(&ustr("Source")),
            Some(&Variant::String("-- rojo source".into()))
        );
        // Base-only property is preserved
        assert_eq!(
            script.properties.get(&ustr("Disabled")),
            Some(&Variant::Bool(true))
        );
    }

    #[test]
    fn merge_model_clones_into_base() {
        let base = WeakDom::new(InstanceBuilder::new("DataModel"));

        let rojo = RojoTree::new(
            InstanceSnapshot::new()
                .name("MyModel")
                .class_name("Folder"),
        );

        let merged = merge_rojo_into_base(base, &rojo).unwrap();

        let root_children = merged.root().children();
        assert_eq!(root_children.len(), 1);

        let model = merged.get_by_ref(root_children[0]).unwrap();
        assert_eq!(model.name, "MyModel");
        assert_eq!(model.class.as_str(), "Folder");
    }
}
