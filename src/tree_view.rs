//! Utiilty that helps redact nondeterministic information from trees so that
//! they can be part of snapshot tests.

use rbx_dom_weak::{
    types::{Ref, Variant},
    Ustr, UstrMap,
};
use rojo_insta_ext::RedactionMap;
use serde::Serialize;

use crate::snapshot::{InstanceMetadata, RojoTree};

/// Adds the given Rojo tree into the redaction map and produces a redacted
/// copy that can be immediately fed to one of Insta's snapshot macros like
/// `assert_snapshot_yaml`.
pub fn view_tree(tree: &RojoTree, redactions: &mut RedactionMap) -> serde_yaml::Value {
    intern_tree(tree, redactions);

    let view = extract_instance_view(tree, tree.get_root_id());
    redactions.redacted_yaml(view)
}

/// Adds the given Rojo tree into the redaction map.
pub fn intern_tree(tree: &RojoTree, redactions: &mut RedactionMap) {
    let root_id = tree.get_root_id();
    redactions.intern(root_id);

    for descendant in tree.descendants(root_id) {
        redactions.intern(descendant.id());
    }
}

/// Copy of data from RojoTree in the right shape to have useful snapshots.
#[derive(Debug, Serialize)]
struct InstanceView {
    id: Ref,
    name: String,
    class_name: Ustr,
    properties: UstrMap<Variant>,
    metadata: InstanceMetadata,
    children: Vec<InstanceView>,
}

fn extract_instance_view(tree: &RojoTree, id: Ref) -> InstanceView {
    let instance = tree.get_instance(id).unwrap();

    InstanceView {
        id: instance.id(),
        name: instance.name().to_owned(),
        class_name: instance.class_name(),
        properties: instance.properties().clone(),
        metadata: instance.metadata().clone(),
        children: instance
            .children()
            .iter()
            .copied()
            .map(|id| extract_instance_view(tree, id))
            .collect(),
    }
}
