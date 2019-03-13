//! Defines a mechanism to compare two RbxTree objects and generate a useful
//! diff if they aren't the same. These methods ignore IDs, which are randomly
//! generated whenever a tree is constructed anyways. This makes matching up
//! pairs of instances that should be the same potentially difficult.
//!
//! It relies on a couple different ideas:
//! - Instances with the same name and class name are matched as the same
//!   instance. See basic_equal for this logic
//! - A path of period-delimited names (like Roblox's GetFullName) should be
//!   enough to debug most issues. If it isn't, we can do something fun like
//!   generate GraphViz graphs.

use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    fmt,
    fs::{self, File},
    hash::Hash,
    path::{Path, PathBuf},
};

use log::error;
use serde_derive::{Serialize, Deserialize};
use rbx_dom_weak::{RbxId, RbxTree};

use librojo::{
    rbx_session::MetadataPerInstance,
    live_session::LiveSession,
    visualize::{VisualizeRbxTree, graphviz_to_svg},
};

use super::snapshot::anonymize_metadata;

/// Marks a 'step' in the test, which will snapshot the session's current
/// RbxTree object and compare it against the saved snapshot if it exists.
pub fn tree_step(step: &str, live_session: &LiveSession, source_path: &Path) {
    let rbx_session = live_session.rbx_session.lock().unwrap();
    let tree = rbx_session.get_tree();

    let project_folder = live_session.root_project().folder_location();
    let metadata = rbx_session.get_all_instance_metadata()
        .iter()
        .map(|(key, meta)| {
            let mut meta = meta.clone();
            anonymize_metadata(project_folder, &mut meta);
            (*key, meta)
        })
        .collect();

    let tree_with_metadata = TreeWithMetadata {
        tree: Cow::Borrowed(&tree),
        metadata: Cow::Owned(metadata),
    };

    match read_tree_by_name(source_path, step) {
        Some(expected) => match trees_equal(&expected, &tree_with_metadata) {
            Ok(_) => {}
            Err(e) => {
                error!("Trees at step '{}' were not equal.\n{}", step, e);

                let expected_gv = format!("{}", VisualizeRbxTree {
                    tree: &expected.tree,
                    metadata: &expected.metadata,
                });

                let actual_gv = format!("{}", VisualizeRbxTree {
                    tree: &tree_with_metadata.tree,
                    metadata: &tree_with_metadata.metadata,
                });

                let output_dir = PathBuf::from("failed-snapshots");
                fs::create_dir_all(&output_dir)
                    .expect("Could not create failed-snapshots directory");

                let expected_basename = format!("{}-{}-expected", live_session.root_project().name, step);
                let actual_basename = format!("{}-{}-actual", live_session.root_project().name, step);

                let mut expected_out = output_dir.join(expected_basename);
                let mut actual_out = output_dir.join(actual_basename);

                match (graphviz_to_svg(&expected_gv), graphviz_to_svg(&actual_gv)) {
                    (Some(expected_svg), Some(actual_svg)) => {
                        expected_out.set_extension("svg");
                        actual_out.set_extension("svg");

                        fs::write(&expected_out, expected_svg)
                            .expect("Couldn't write expected SVG");

                        fs::write(&actual_out, actual_svg)
                            .expect("Couldn't write actual SVG");
                    }
                    _ => {
                        expected_out.set_extension("gv");
                        actual_out.set_extension("gv");

                        fs::write(&expected_out, expected_gv)
                            .expect("Couldn't write expected GV");

                        fs::write(&actual_out, actual_gv)
                            .expect("Couldn't write actual GV");
                    }
                }

                error!("Output at {} and {}", expected_out.display(), actual_out.display());

                panic!("Tree mismatch at step '{}'", step);
            }
        }
        None => {
            write_tree_by_name(source_path, step, &tree_with_metadata);
        }
    }
}

fn new_cow_map<K: Clone + Eq + Hash, V: Clone>() -> Cow<'static, HashMap<K, V>> {
    Cow::Owned(HashMap::new())
}

#[derive(Debug, Serialize, Deserialize)]
struct TreeWithMetadata<'a> {
    #[serde(flatten)]
    pub tree: Cow<'a, RbxTree>,

    #[serde(default = "new_cow_map")]
    pub metadata: Cow<'a, HashMap<RbxId, MetadataPerInstance>>,
}

fn read_tree_by_name(path: &Path, identifier: &str) -> Option<TreeWithMetadata<'static>> {
    let mut file_path = path.join(identifier);
    file_path.set_extension("tree.json");

    let contents = fs::read(&file_path).ok()?;
    let tree: TreeWithMetadata = serde_json::from_slice(&contents)
        .expect("Could not deserialize tree");

    Some(tree)
}

fn write_tree_by_name(path: &Path, identifier: &str, tree: &TreeWithMetadata) {
    let mut file_path = path.join(identifier);
    file_path.set_extension("tree.json");

    let mut file = File::create(file_path)
        .expect("Could not open file to write tree");

    serde_json::to_writer_pretty(&mut file, tree)
        .expect("Could not serialize tree to file");
}

#[derive(Debug)]
struct TreeMismatch {
    pub path: Cow<'static, str>,
    pub detail: Cow<'static, str>,
}

impl TreeMismatch {
    pub fn new<'a, A: Into<Cow<'a, str>>, B: Into<Cow<'a, str>>>(path: A, detail: B) -> TreeMismatch {
        TreeMismatch {
            path: Cow::Owned(path.into().into_owned()),
            detail: Cow::Owned(detail.into().into_owned()),
        }
    }

    pub fn add_parent(mut self, name: &str) -> TreeMismatch {
        self.path.to_mut().insert(0, '.');
        self.path.to_mut().insert_str(0, name);

        TreeMismatch {
            path: self.path,
            detail: self.detail,
        }
    }
}

impl fmt::Display for TreeMismatch {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        writeln!(formatter, "Tree mismatch at path {}", self.path)?;
        writeln!(formatter, "{}", self.detail)
    }
}

fn trees_equal(
    left_tree: &TreeWithMetadata,
    right_tree: &TreeWithMetadata,
) -> Result<(), TreeMismatch> {
    let left_id = left_tree.tree.get_root_id();
    let right_id = right_tree.tree.get_root_id();

    instances_equal(left_tree, left_id, right_tree, right_id)
}

fn instances_equal(
    left_tree: &TreeWithMetadata,
    left_id: RbxId,
    right_tree: &TreeWithMetadata,
    right_id: RbxId,
) -> Result<(), TreeMismatch> {
    basic_equal(left_tree, left_id, right_tree, right_id)?;
    properties_equal(left_tree, left_id, right_tree, right_id)?;
    children_equal(left_tree, left_id, right_tree, right_id)?;
    metadata_equal(left_tree, left_id, right_tree, right_id)
}

fn basic_equal(
    left_tree: &TreeWithMetadata,
    left_id: RbxId,
    right_tree: &TreeWithMetadata,
    right_id: RbxId,
) -> Result<(), TreeMismatch> {
    let left_instance = left_tree.tree.get_instance(left_id)
        .expect("ID did not exist in left tree");

    let right_instance = right_tree.tree.get_instance(right_id)
        .expect("ID did not exist in right tree");

    if left_instance.name != right_instance.name {
        let message = format!("Name did not match ('{}' vs '{}')", left_instance.name, right_instance.name);

        return Err(TreeMismatch::new(&left_instance.name, message));
    }

    if left_instance.class_name != right_instance.class_name {
        let message = format!("Class name did not match ('{}' vs '{}')", left_instance.class_name, right_instance.class_name);

        return Err(TreeMismatch::new(&left_instance.name, message));
    }

    Ok(())
}

fn properties_equal(
    left_tree: &TreeWithMetadata,
    left_id: RbxId,
    right_tree: &TreeWithMetadata,
    right_id: RbxId,
) -> Result<(), TreeMismatch> {
    let left_instance = left_tree.tree.get_instance(left_id)
        .expect("ID did not exist in left tree");

    let right_instance = right_tree.tree.get_instance(right_id)
        .expect("ID did not exist in right tree");

    let mut visited = HashSet::new();

    for (key, left_value) in &left_instance.properties {
        visited.insert(key);

        let right_value = right_instance.properties.get(key);

        if Some(left_value) != right_value {
            let message = format!(
                "Property {}:\n\tLeft: {:?}\n\tRight: {:?}",
                key,
                Some(left_value),
                right_value,
            );

            return Err(TreeMismatch::new(&left_instance.name, message));
        }
    }

    for (key, right_value) in &right_instance.properties {
        if visited.contains(key) {
            continue;
        }

        let left_value = left_instance.properties.get(key);

        if left_value != Some(right_value) {
            let message = format!(
                "Property {}:\n\tLeft: {:?}\n\tRight: {:?}",
                key,
                left_value,
                Some(right_value),
            );

            return Err(TreeMismatch::new(&left_instance.name, message));
        }
    }

    Ok(())
}

fn children_equal(
    left_tree: &TreeWithMetadata,
    left_id: RbxId,
    right_tree: &TreeWithMetadata,
    right_id: RbxId,
) -> Result<(), TreeMismatch> {
    let left_instance = left_tree.tree.get_instance(left_id)
        .expect("ID did not exist in left tree");

    let right_instance = right_tree.tree.get_instance(right_id)
        .expect("ID did not exist in right tree");

    let left_children = left_instance.get_children_ids();
    let right_children = right_instance.get_children_ids();

    if left_children.len() != right_children.len() {
        return Err(TreeMismatch::new(&left_instance.name, "Instances had different numbers of children"));
    }

    for (left_child_id, right_child_id) in left_children.iter().zip(right_children) {
        instances_equal(left_tree, *left_child_id, right_tree, *right_child_id)
            .map_err(|e| e.add_parent(&left_instance.name))?;
    }

    Ok(())
}

fn metadata_equal(
    left_tree: &TreeWithMetadata,
    left_id: RbxId,
    right_tree: &TreeWithMetadata,
    right_id: RbxId,
) -> Result<(), TreeMismatch> {
    let left_meta = left_tree.metadata.get(&left_id);
    let right_meta = right_tree.metadata.get(&right_id);

    if left_meta != right_meta {
        let left_instance = left_tree.tree.get_instance(left_id)
            .expect("Left instance didn't exist in tree");

        let message = format!(
            "Metadata mismatch:\n\tLeft: {:?}\n\tRight: {:?}",
            left_meta,
            right_meta,
        );

        return Err(TreeMismatch::new(&left_instance.name, message));
    }

    Ok(())
}