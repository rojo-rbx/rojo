use std::fmt;
use rbx_tree::{RbxTree, RbxId};

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

pub struct VisualizeTree<'a>(pub &'a RbxTree);

impl<'a> fmt::Display for VisualizeTree<'a> {
    fn fmt(&self, output: &mut fmt::Formatter) -> fmt::Result {
        writeln!(output, "{}", GRAPHVIZ_HEADER)?;

        visualize_node(self.0, self.0.get_root_id(), output)?;

        writeln!(output, "}}")?;

        Ok(())
    }
}

fn visualize_node(tree: &RbxTree, id: RbxId, output: &mut fmt::Formatter) -> fmt::Result {
    let node = tree.get_instance(id).unwrap();

    writeln!(output, "    \"{}\" [label=\"{}\"]", id, node.name)?;

    for &child_id in node.get_children_ids() {
        writeln!(output, "    \"{}\" -> \"{}\"", id, child_id)?;
        visualize_node(tree, child_id, output)?;
    }

    Ok(())
}