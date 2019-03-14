use std::{
    collections::HashMap,
    fmt,
    io::Write,
    path::Path,
    process::{Command, Stdio},
};

use log::warn;
use rbx_dom_weak::{RbxTree, RbxId};

use crate::{
    imfs::{Imfs, ImfsItem},
    rbx_session::RbxSession,
    web::api::PublicInstanceMetadata,
    rbx_session::MetadataPerInstance,
};

static GRAPHVIZ_HEADER: &str = r#"
digraph RojoTree {
    rankdir = "LR";
    graph [
        ranksep = "0.7",
        nodesep = "0.5",
    ];
    node [
        fontname = "Hack",
        shape = "record",
    ];
"#;

/// Compiles DOT source to SVG by invoking dot on the command line.
pub fn graphviz_to_svg(source: &str) -> Option<String> {
    let command = Command::new("dot")
        .arg("-Tsvg")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn();

    let mut child = match command {
        Ok(child) => child,
        Err(_) => {
            warn!("Failed to spawn GraphViz process to visualize current state.");
            warn!("If you want pretty graphs, install GraphViz and make sure 'dot' is on your PATH!");
            return None;
        },
    };

    {
        let stdin = child.stdin.as_mut().expect("Failed to open stdin");
        stdin.write_all(source.as_bytes()).expect("Failed to write to stdin");
    }

    let output = child.wait_with_output().expect("Failed to read stdout");
    Some(String::from_utf8(output.stdout).expect("Failed to parse stdout as UTF-8"))
}

pub struct VisualizeRbxTree<'a, 'b> {
    pub tree: &'a RbxTree,
    pub metadata: &'b HashMap<RbxId, MetadataPerInstance>,
}

impl<'a, 'b> fmt::Display for VisualizeRbxTree<'a, 'b> {
    fn fmt(&self, output: &mut fmt::Formatter) -> fmt::Result {
        writeln!(output, "{}", GRAPHVIZ_HEADER)?;

        visualize_instance(&self.tree, self.tree.get_root_id(), &self.metadata, output)?;

        writeln!(output, "}}")
    }
}

/// A Display wrapper struct to visualize an RbxSession as SVG.
pub struct VisualizeRbxSession<'a>(pub &'a RbxSession);

impl<'a> fmt::Display for VisualizeRbxSession<'a> {
    fn fmt(&self, output: &mut fmt::Formatter) -> fmt::Result {
        writeln!(output, "{}", VisualizeRbxTree {
            tree: self.0.get_tree(),
            metadata: self.0.get_all_instance_metadata(),
        })
    }
}

fn visualize_instance(
    tree: &RbxTree,
    id: RbxId,
    metadata: &HashMap<RbxId, MetadataPerInstance>,
    output: &mut fmt::Formatter,
) -> fmt::Result {
    let instance = tree.get_instance(id).unwrap();

    let mut instance_label = format!("{}|{}|{}", instance.name, instance.class_name, id);

    if let Some(session_metadata) = metadata.get(&id) {
        let metadata = PublicInstanceMetadata::from_session_metadata(session_metadata);
        instance_label.push('|');
        instance_label.push_str(&serde_json::to_string(&metadata).unwrap());
    }

    instance_label = instance_label
        .replace("\"", "&quot;")
        .replace("{", "\\{")
        .replace("}", "\\}");

    writeln!(output, "    \"{}\" [label=\"{}\"]", id, instance_label)?;

    for &child_id in instance.get_children_ids() {
        writeln!(output, "    \"{}\" -> \"{}\"", id, child_id)?;
        visualize_instance(tree, child_id, metadata, output)?;
    }

    Ok(())
}

/// A Display wrapper struct to visualize an Imfs as SVG.
pub struct VisualizeImfs<'a>(pub &'a Imfs);

impl<'a> fmt::Display for VisualizeImfs<'a> {
    fn fmt(&self, output: &mut fmt::Formatter) -> fmt::Result {
        writeln!(output, "{}", GRAPHVIZ_HEADER)?;

        for root_path in self.0.get_roots() {
            visualize_root_path(self.0, root_path, output)?;
        }

        writeln!(output, "}}")?;

        Ok(())
    }
}

fn normalize_name(path: &Path) -> String {
    path.to_str().unwrap().replace("\\", "/")
}

fn visualize_root_path(imfs: &Imfs, path: &Path, output: &mut fmt::Formatter) -> fmt::Result {
    let normalized_name = normalize_name(path);
    let item = imfs.get(path).unwrap();

    writeln!(output, "    \"{}\"", normalized_name)?;

    match item {
        ImfsItem::File(_) => {},
        ImfsItem::Directory(directory) => {
            for child_path in &directory.children {
                writeln!(output, "    \"{}\" -> \"{}\"", normalized_name, normalize_name(child_path))?;
                visualize_path(imfs, child_path, output)?;
            }
        },
    }

    Ok(())
}

fn visualize_path(imfs: &Imfs, path: &Path, output: &mut fmt::Formatter) -> fmt::Result {
    let normalized_name = normalize_name(path);
    let short_name = path.file_name().unwrap().to_string_lossy();
    let item = imfs.get(path).unwrap();

    writeln!(output, "    \"{}\" [label = \"{}\"]", normalized_name, short_name)?;

    match item {
        ImfsItem::File(_) => {},
        ImfsItem::Directory(directory) => {
            for child_path in &directory.children {
                writeln!(output, "    \"{}\" -> \"{}\"", normalized_name, normalize_name(child_path))?;
                visualize_path(imfs, child_path, output)?;
            }
        },
    }

    Ok(())
}