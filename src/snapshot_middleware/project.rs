use std::{borrow::Cow, collections::HashMap};

use rbx_dom_weak::{RbxId, RbxTree};
use rbx_reflection::try_resolve_value;

use crate::{
    imfs::{FsErrorKind, Imfs, ImfsEntry, ImfsFetcher},
    project::{Project, ProjectNode},
    snapshot::{InstanceMetadata, InstanceSnapshot},
};

use super::{
    middleware::{SnapshotFileResult, SnapshotInstanceResult, SnapshotMiddleware},
    snapshot_from_imfs,
};

pub struct SnapshotProject;

impl SnapshotMiddleware for SnapshotProject {
    fn from_imfs<F: ImfsFetcher>(
        imfs: &mut Imfs<F>,
        entry: &ImfsEntry,
    ) -> SnapshotInstanceResult<'static> {
        if entry.is_directory() {
            let project_path = entry.path().join("default.project.json");

            match imfs.get(project_path) {
                Err(ref err) if err.kind() == FsErrorKind::NotFound => {}
                Err(err) => return Err(err),
                Ok(entry) => return SnapshotProject::from_imfs(imfs, &entry),
            }
        }

        if !entry.path().to_string_lossy().ends_with(".project.json") {
            return Ok(None);
        }

        let project = Project::load_from_slice(entry.contents(imfs)?, entry.path())
            .expect("Invalid project file");

        snapshot_project_node(&project.name, &project.tree, imfs)
    }

    fn from_instance(_tree: &RbxTree, _id: RbxId) -> SnapshotFileResult {
        // TODO: Supporting turning instances into projects
        None
    }
}

fn snapshot_project_node<F: ImfsFetcher>(
    instance_name: &str,
    node: &ProjectNode,
    imfs: &mut Imfs<F>,
) -> SnapshotInstanceResult<'static> {
    let ignore_unknown_instances = node.ignore_unknown_instances.unwrap_or(node.path.is_none());

    let name = Cow::Owned(instance_name.to_owned());
    let mut class_name = node
        .class_name
        .as_ref()
        .map(|name| Cow::Owned(name.clone()));
    let mut properties = HashMap::new();
    let mut children = Vec::new();

    if let Some(path) = &node.path {
        let entry = imfs.get(path)?;

        if let Some(snapshot) = snapshot_from_imfs(imfs, &entry)? {
            // If a class name was already specified, then it'll override the
            // class name of this snapshot ONLY if it's a Folder.
            //
            // This restriction is in place to prevent applying properties to
            // instances that don't make sense. The primary use-case for using
            // $className and $path at the same time is to use a directory as a
            // service in a place file.
            class_name = match class_name {
                Some(class_name) => {
                    if snapshot.class_name == "Folder" {
                        Some(class_name)
                    } else {
                        // TODO: Turn this into an error object.
                        panic!("If $className and $path are specified, $path must yield an instance of class Folder");
                    }
                }
                None => Some(snapshot.class_name),
            };

            // Properties from the snapshot are pulled in unchanged, and
            // overridden by properties set on the project node.
            properties.reserve(snapshot.properties.len());
            for (key, value) in snapshot.properties.into_iter() {
                properties.insert(key, value);
            }

            // The snapshot's children will be merged with the children defined
            // in the project node, if there are any.
            children.reserve(snapshot.children.len());
            for child in snapshot.children.into_iter() {
                children.push(child);
            }
        } else {
            // TODO: Should this issue an error instead?
            log::warn!(
                "$path referred to a path that could not be turned into an instance by Rojo"
            );
        }
    }

    let class_name = class_name
        // TODO: Turn this into an error object.
        .expect("$className or $path must be specified");

    for (child_name, child_project_node) in &node.children {
        if let Some(child) = snapshot_project_node(child_name, child_project_node, imfs)? {
            children.push(child);
        }
    }

    for (key, value) in &node.properties {
        let resolved_value = try_resolve_value(&class_name, key, value)
            .expect("TODO: Properly handle value resolution errors");

        properties.insert(key.clone(), resolved_value);
    }

    Ok(Some(InstanceSnapshot {
        snapshot_id: None,
        metadata: InstanceMetadata {
            ignore_unknown_instances,
            ..Default::default() // TODO: Fill out remaining metadata
        },
        name,
        class_name,
        properties,
        children,
    }))
}

#[cfg(test)]
mod test {
    use super::*;

    use maplit::hashmap;
    use rbx_dom_weak::RbxValue;

    use crate::imfs::{ImfsDebug, ImfsSnapshot, NoopFetcher};

    #[test]
    fn project_from_folder() {
        let _ = env_logger::try_init();

        let mut imfs = Imfs::new(NoopFetcher);
        let dir = ImfsSnapshot::dir(hashmap! {
            "default.project.json" => ImfsSnapshot::file(r#"
                {
                    "name": "indirect-project",
                    "tree": {
                        "$className": "Folder"
                    }
                }
            "#),
        });

        imfs.debug_load_snapshot("/foo", dir);

        let entry = imfs.get("/foo").unwrap();
        let instance_snapshot = SnapshotProject::from_imfs(&mut imfs, &entry)
            .expect("snapshot error")
            .expect("snapshot returned no instances");

        assert_eq!(instance_snapshot.name, "indirect-project");
        assert_eq!(instance_snapshot.class_name, "Folder");
        assert_eq!(instance_snapshot.properties, HashMap::new());
        assert_eq!(instance_snapshot.children, Vec::new());
    }

    #[test]
    fn project_from_direct_file() {
        let _ = env_logger::try_init();

        let mut imfs = Imfs::new(NoopFetcher);
        let dir = ImfsSnapshot::dir(hashmap! {
            "hello.project.json" => ImfsSnapshot::file(r#"
                {
                    "name": "direct-project",
                    "tree": {
                        "$className": "Model"
                    }
                }
            "#),
        });

        imfs.debug_load_snapshot("/foo", dir);

        let entry = imfs.get("/foo/hello.project.json").unwrap();
        let instance_snapshot = SnapshotProject::from_imfs(&mut imfs, &entry)
            .expect("snapshot error")
            .expect("snapshot returned no instances");

        assert_eq!(instance_snapshot.name, "direct-project");
        assert_eq!(instance_snapshot.class_name, "Model");
        assert_eq!(instance_snapshot.properties, HashMap::new());
        assert_eq!(instance_snapshot.children, Vec::new());
    }

    #[test]
    fn project_with_resolved_properties() {
        let _ = env_logger::try_init();

        let mut imfs = Imfs::new(NoopFetcher);
        let dir = ImfsSnapshot::dir(hashmap! {
            "default.project.json" => ImfsSnapshot::file(r#"
                {
                    "name": "resolved-properties",
                    "tree": {
                        "$className": "StringValue",
                        "$properties": {
                            "Value": {
                                "Type": "String",
                                "Value": "Hello, world!"
                            }
                        }
                    }
                }
            "#),
        });

        imfs.debug_load_snapshot("/foo", dir);

        let entry = imfs.get("/foo").unwrap();
        let instance_snapshot = SnapshotProject::from_imfs(&mut imfs, &entry)
            .expect("snapshot error")
            .expect("snapshot returned no instances");

        assert_eq!(instance_snapshot.name, "resolved-properties");
        assert_eq!(instance_snapshot.class_name, "StringValue");
        assert_eq!(
            instance_snapshot.properties,
            hashmap! {
                "Value".to_owned() => RbxValue::String {
                    value: "Hello, world!".to_owned(),
                },
            }
        );
        assert_eq!(instance_snapshot.children, Vec::new());
    }

    #[test]
    fn project_with_unresolved_properties() {
        let _ = env_logger::try_init();

        let mut imfs = Imfs::new(NoopFetcher);
        let dir = ImfsSnapshot::dir(hashmap! {
            "default.project.json" => ImfsSnapshot::file(r#"
                {
                    "name": "unresolved-properties",
                    "tree": {
                        "$className": "StringValue",
                        "$properties": {
                            "Value": "Hi!"
                        }
                    }
                }
            "#),
        });

        imfs.debug_load_snapshot("/foo", dir);

        let entry = imfs.get("/foo").unwrap();
        let instance_snapshot = SnapshotProject::from_imfs(&mut imfs, &entry)
            .expect("snapshot error")
            .expect("snapshot returned no instances");

        assert_eq!(instance_snapshot.name, "unresolved-properties");
        assert_eq!(instance_snapshot.class_name, "StringValue");
        assert_eq!(
            instance_snapshot.properties,
            hashmap! {
                "Value".to_owned() => RbxValue::String {
                    value: "Hi!".to_owned(),
                },
            }
        );
        assert_eq!(instance_snapshot.children, Vec::new());
    }

    #[test]
    fn project_with_children() {
        let _ = env_logger::try_init();

        let mut imfs = Imfs::new(NoopFetcher);
        let dir = ImfsSnapshot::dir(hashmap! {
            "default.project.json" => ImfsSnapshot::file(r#"
                {
                    "name": "children",
                    "tree": {
                        "$className": "Folder",

                        "Child": {
                            "$className": "Model"
                        }
                    }
                }
            "#),
        });

        imfs.debug_load_snapshot("/foo", dir);

        let entry = imfs.get("/foo").unwrap();
        let instance_snapshot = SnapshotProject::from_imfs(&mut imfs, &entry)
            .expect("snapshot error")
            .expect("snapshot returned no instances");

        assert_eq!(instance_snapshot.name, "children");
        assert_eq!(instance_snapshot.class_name, "Folder");
        assert_eq!(instance_snapshot.properties, HashMap::new());
        assert_eq!(instance_snapshot.children.len(), 1);

        let child = &instance_snapshot.children[0];
        assert_eq!(child.name, "Child");
        assert_eq!(child.class_name, "Model");
        assert_eq!(child.properties, HashMap::new());
        assert_eq!(child.children, Vec::new());
    }

    #[test]
    fn project_with_path_to_txt() {
        let _ = env_logger::try_init();

        let mut imfs = Imfs::new(NoopFetcher);
        let dir = ImfsSnapshot::dir(hashmap! {
            "default.project.json" => ImfsSnapshot::file(r#"
                {
                    "name": "path-project",
                    "tree": {
                        "$path": "other.txt"
                    }
                }
            "#),
            "other.txt" => ImfsSnapshot::file("Hello, world!"),
        });

        imfs.debug_load_snapshot("/foo", dir);

        let entry = imfs.get("/foo").unwrap();
        let instance_snapshot = SnapshotProject::from_imfs(&mut imfs, &entry)
            .expect("snapshot error")
            .expect("snapshot returned no instances");

        assert_eq!(instance_snapshot.name, "path-project");
        assert_eq!(instance_snapshot.class_name, "StringValue");
        assert_eq!(
            instance_snapshot.properties,
            hashmap! {
                "Value".to_owned() => RbxValue::String {
                    value: "Hello, world!".to_owned(),
                },
            }
        );
        assert_eq!(instance_snapshot.children, Vec::new());
    }

    #[test]
    fn project_with_path_to_project() {
        let _ = env_logger::try_init();

        let mut imfs = Imfs::new(NoopFetcher);
        let dir = ImfsSnapshot::dir(hashmap! {
            "default.project.json" => ImfsSnapshot::file(r#"
                {
                    "name": "path-project",
                    "tree": {
                        "$path": "other.project.json"
                    }
                }
            "#),
            "other.project.json" => ImfsSnapshot::file(r#"
                {
                    "name": "other-project",
                    "tree": {
                        "$className": "Model"
                    }
                }
            "#),
        });

        imfs.debug_load_snapshot("/foo", dir);

        let entry = imfs.get("/foo").unwrap();
        let instance_snapshot = SnapshotProject::from_imfs(&mut imfs, &entry)
            .expect("snapshot error")
            .expect("snapshot returned no instances");

        assert_eq!(instance_snapshot.name, "path-project");
        assert_eq!(instance_snapshot.class_name, "Model");
        assert_eq!(instance_snapshot.properties, HashMap::new());
        assert_eq!(instance_snapshot.children, Vec::new());
    }

    #[test]
    fn project_with_path_to_project_with_children() {
        let _ = env_logger::try_init();

        let mut imfs = Imfs::new(NoopFetcher);
        let dir = ImfsSnapshot::dir(hashmap! {
            "default.project.json" => ImfsSnapshot::file(r#"
                {
                    "name": "path-child-project",
                    "tree": {
                        "$path": "other.project.json"
                    }
                }
            "#),
            "other.project.json" => ImfsSnapshot::file(r#"
                {
                    "name": "other-project",
                    "tree": {
                        "$className": "Folder",

                        "SomeChild": {
                            "$className": "Model"
                        }
                    }
                }
            "#),
        });

        imfs.debug_load_snapshot("/foo", dir);

        let entry = imfs.get("/foo").unwrap();
        let instance_snapshot = SnapshotProject::from_imfs(&mut imfs, &entry)
            .expect("snapshot error")
            .expect("snapshot returned no instances");

        assert_eq!(instance_snapshot.name, "path-child-project");
        assert_eq!(instance_snapshot.class_name, "Folder");
        assert_eq!(instance_snapshot.properties, HashMap::new());
        assert_eq!(instance_snapshot.children.len(), 1);

        let child = &instance_snapshot.children[0];
        assert_eq!(child.name, "SomeChild");
        assert_eq!(child.class_name, "Model");
        assert_eq!(child.properties, HashMap::new());
        assert_eq!(child.children, Vec::new());
    }

    /// Ensures that if a property is defined both in the resulting instance
    /// from $path and also in $properties, that the $properties value takes
    /// precedence.
    #[test]
    fn project_path_property_overrides() {
        let _ = env_logger::try_init();

        let mut imfs = Imfs::new(NoopFetcher);
        let dir = ImfsSnapshot::dir(hashmap! {
            "default.project.json" => ImfsSnapshot::file(r#"
                {
                    "name": "path-property-override",
                    "tree": {
                        "$path": "other.project.json",
                        "$properties": {
                            "Value": "Changed"
                        }
                    }
                }
            "#),
            "other.project.json" => ImfsSnapshot::file(r#"
                {
                    "name": "other-project",
                    "tree": {
                        "$className": "StringValue",
                        "$properties": {
                            "Value": "Original"
                        }
                    }
                }
            "#),
        });

        imfs.debug_load_snapshot("/foo", dir);

        let entry = imfs.get("/foo").unwrap();
        let instance_snapshot = SnapshotProject::from_imfs(&mut imfs, &entry)
            .expect("snapshot error")
            .expect("snapshot returned no instances");

        assert_eq!(instance_snapshot.name, "path-property-override");
        assert_eq!(instance_snapshot.class_name, "StringValue");
        assert_eq!(
            instance_snapshot.properties,
            hashmap! {
                "Value".to_owned() => RbxValue::String {
                    value: "Changed".to_owned(),
                },
            }
        );
        assert_eq!(instance_snapshot.children, Vec::new());
    }
}
