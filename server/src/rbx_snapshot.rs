use std::{
    str,
    borrow::Cow,
    collections::HashMap,
};

use rbx_tree::{RbxTree, RbxId};

use crate::{
    imfs::{Imfs, ImfsItem, ImfsFile, ImfsDirectory},
};

pub struct RbxSnapshotInstance<'a> {
    name: String,
    class_name: String,
    properties: HashMap<String, RbxSnapshotValue<'a>>,
    children: Vec<RbxSnapshotInstance<'a>>,
}

pub enum RbxSnapshotValue<'a> {
    String(Cow<'a, str>),
}

pub fn reify(snapshot: RbxSnapshotInstance, tree: &mut RbxTree, parent_id: RbxId) {
    unimplemented!()
}

pub fn render<'a>(imfs: &'a Imfs, imfs_item: &'a ImfsItem) -> RbxSnapshotInstance<'a> {
    match imfs_item {
        ImfsItem::File(file) => {
            let name = file.path.file_stem().unwrap().to_str().unwrap();
            let source = str::from_utf8(&file.contents).unwrap();
            let mut properties = HashMap::new();
            properties.insert("Source".to_string(), RbxSnapshotValue::String(Cow::Borrowed(source)));

            RbxSnapshotInstance {
                name: name.to_string(),
                class_name: "ModuleScript".to_string(),
                properties,
                children: Vec::new(),
            }
        },
        ImfsItem::Directory(directory) => {
            let name = directory.path.file_name().unwrap().to_str().unwrap();
            let mut children = Vec::new();

            for child_path in &directory.children {
                let child_item = imfs.get(child_path).unwrap();
                children.push(render(imfs, child_item));
            }

            RbxSnapshotInstance {
                name: name.to_string(),
                class_name: "Folder".to_string(),
                properties: HashMap::new(),
                children,
            }
        },
    }
}