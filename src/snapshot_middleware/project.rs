use std::{borrow::Cow, path::Path};

use anyhow::{bail, Context};
use memofs::Vfs;
use rbx_dom_weak::{
    types::{Attributes, Ref},
    ustr, HashMapExt as _, Ustr, UstrMap,
};
use rbx_reflection::ClassTag;

use crate::{
    project::{PathNode, Project, ProjectNode},
    snapshot::{
        InstanceContext, InstanceMetadata, InstanceSnapshot, InstigatingSource, PathIgnoreRule,
        SyncRule,
    },
    RojoRef,
};

use super::{emit_legacy_scripts_default, snapshot_from_vfs};

pub fn snapshot_project(
    context: &InstanceContext,
    vfs: &Vfs,
    path: &Path,
    name: &str,
) -> anyhow::Result<Option<InstanceSnapshot>> {
    let project = Project::load_exact(vfs, path, Some(name))
        .with_context(|| format!("File was not a valid Rojo project: {}", path.display()))?;
    let project_name = match project.name.as_deref() {
        Some(name) => name,
        None => panic!("Project is missing a name"),
    };

    let mut context = context.clone();
    context.clear_sync_rules();

    let rules = project.glob_ignore_paths.iter().map(|glob| PathIgnoreRule {
        glob: glob.clone(),
        base_path: project.folder_location().to_path_buf(),
    });

    let sync_rules = project.sync_rules.iter().map(|rule| SyncRule {
        base_path: project.folder_location().to_path_buf(),
        ..rule.clone()
    });

    context.add_sync_rules(sync_rules);
    context.add_path_ignore_rules(rules);
    context.set_emit_legacy_scripts(
        project
            .emit_legacy_scripts
            .or_else(emit_legacy_scripts_default)
            .unwrap(),
    );

    match snapshot_project_node(&context, path, project_name, &project.tree, vfs, None)? {
        Some(found_snapshot) => {
            let mut snapshot = found_snapshot;
            // Setting the instigating source to the project file path is a little
            // coarse.
            //
            // Ideally, we'd only snapshot the project file if the project file
            // actually changed. Because Rojo only has the concept of one
            // relevant path -> snapshot path mapping per instance, we pick the more
            // conservative approach of snapshotting the project file if any
            // relevant paths changed.
            snapshot.metadata.instigating_source = Some(path.to_path_buf().into());

            // Mark this snapshot (the root node of the project file) as being
            // related to the project file.
            //
            // We SHOULD NOT mark the project file as a relevant path for any
            // nodes that aren't roots. They'll be updated as part of the project
            // file being updated.
            snapshot.metadata.relevant_paths.push(path.to_path_buf());

            Ok(Some(snapshot))
        }
        None => Ok(None),
    }
}

pub fn snapshot_project_node(
    context: &InstanceContext,
    project_path: &Path,
    instance_name: &str,
    node: &ProjectNode,
    vfs: &Vfs,
    parent_class: Option<&str>,
) -> anyhow::Result<Option<InstanceSnapshot>> {
    let project_folder = project_path.parent().unwrap();

    let mut class_name_from_path = None;

    let name = Cow::Owned(instance_name.to_owned());
    let mut properties = UstrMap::new();
    let mut children = Vec::new();
    let mut metadata = InstanceMetadata::new().context(context);

    if let Some(path_node) = &node.path {
        let path = path_node.path();

        // If the path specified in the project is relative, we assume it's
        // relative to the folder that the project is in, project_folder.
        let full_path = if path.is_relative() {
            Cow::Owned(project_folder.join(path))
        } else {
            Cow::Borrowed(path)
        };

        if let Some(snapshot) = snapshot_from_vfs(context, vfs, &full_path)? {
            class_name_from_path = Some(snapshot.class_name);

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

            // Take the snapshot's metadata as-is, which will be mutated later
            // on.
            metadata = snapshot.metadata;
        }
    }

    let class_name_from_inference = infer_class_name(&name, parent_class);

    let class_name = match (
        node.class_name,
        class_name_from_path,
        class_name_from_inference,
        &node.path,
    ) {
        // These are the easy, happy paths!
        (Some(project), None, None, _) => project,
        (None, Some(path), None, _) => path,
        (None, None, Some(inference), _) => inference,

        // If the user specifies a class name, but there's an inferred class
        // name, we prefer the name listed explicitly by the user.
        (Some(project), None, Some(_), _) => project,

        // If the user has a $path pointing to a folder and we're able to infer
        // a class name, let's use the inferred name. If the path we're pointing
        // to isn't a folder, though, that's a user error.
        (None, Some(path), Some(inference), _) => {
            if path == "Folder" {
                inference
            } else {
                path
            }
        }

        (Some(project), Some(path), _, _) => {
            if path == "Folder" {
                project
            } else {
                bail!(
                    "ClassName for Instance \"{}\" was specified in both the project file (as \"{}\") and from the filesystem (as \"{}\").\n\
                     If $className and $path are both set, $path must refer to a Folder.
                     \n\
                     Project path: {}\n\
                     Filesystem path: {}\n",
                    instance_name,
                    project,
                    path,
                    project_path.display(),
                    node.path.as_ref().unwrap().path().display()
                );
            }
        }

        (None, None, None, Some(PathNode::Optional(_))) => {
            return Ok(None);
        }

        (_, None, _, Some(PathNode::Required(path))) => {
            anyhow::bail!(
                "Rojo project referred to a file using $path that could not be turned into a Roblox Instance by Rojo.\n\
                Check that the file exists and is a file type known by Rojo.\n\
                \n\
                Project path: {}\n\
                File $path: {}",
                project_path.display(),
                path.display(),
            );
        }

        (None, None, None, None) => {
            bail!(
                "Instance \"{}\" is missing some required information.\n\
                 One of the following must be true:\n\
                 - $className must be set to the name of a Roblox class\n\
                 - $path must be set to a path of an instance\n\
                 - The instance must be a known service, like ReplicatedStorage\n\
                 \n\
                 Project path: {}",
                instance_name,
                project_path.display(),
            );
        }
    };

    for (child_name, child_project_node) in &node.children {
        if let Some(child) = snapshot_project_node(
            context,
            project_path,
            child_name,
            child_project_node,
            vfs,
            Some(&class_name),
        )? {
            children.push(child);
        }
    }

    for (key, unresolved) in &node.properties {
        let value = unresolved
            .clone()
            .resolve(&class_name, key)
            .with_context(|| {
                format!(
                    "Unresolvable property in project at path {}",
                    project_path.display()
                )
            })?;

        match key.as_str() {
            "Name" | "Parent" => {
                log::warn!(
                    "Property '{}' cannot be set manually, ignoring. Attempted to set in '{}' at {}",
                    key,
                    instance_name,
                    project_path.display()
                );
                continue;
            }

            _ => {}
        }

        properties.insert(*key, value);
    }

    if !node.attributes.is_empty() {
        let mut attributes = Attributes::new();

        for (key, unresolved) in &node.attributes {
            let value = unresolved.clone().resolve_unambiguous().with_context(|| {
                format!(
                    "Unresolvable attribute in project at path {}",
                    project_path.display()
                )
            })?;

            attributes.insert(key.clone(), value);
        }

        properties.insert("Attributes".into(), attributes.into());
    }

    // If the user specified $ignoreUnknownInstances, overwrite the existing
    // value.
    //
    // If the user didn't specify it AND $path was not specified (meaning
    // there's no existing value we'd be stepping on from a project file or meta
    // file), set it to true.
    if let Some(ignore) = node.ignore_unknown_instances {
        metadata.ignore_unknown_instances = ignore;
    } else if node.path.is_none() {
        // TODO: Introduce a strict mode where $ignoreUnknownInstances is never
        // set implicitly.
        metadata.ignore_unknown_instances = true;
    }

    if let Some(id) = &node.id {
        metadata.specified_id = Some(RojoRef::new(id.clone()))
    }

    metadata.instigating_source = Some(InstigatingSource::ProjectNode(
        project_path.to_path_buf(),
        instance_name.to_string(),
        Box::new(node.clone()),
        parent_class.map(|name| name.to_owned()),
    ));

    Ok(Some(InstanceSnapshot {
        snapshot_id: Ref::none(),
        name,
        class_name,
        properties,
        children,
        metadata,
    }))
}

fn infer_class_name(name: &str, parent_class: Option<&str>) -> Option<Ustr> {
    // If className wasn't defined from another source, we may be able
    // to infer one.

    let parent_class = parent_class?;

    if parent_class == "DataModel" {
        // Members of DataModel with names that match known services are
        // probably supposed to be those services.

        let descriptor = rbx_reflection_database::get().unwrap().classes.get(name)?;

        if descriptor.tags.contains(&ClassTag::Service) {
            return Some(ustr(name));
        }
    } else if parent_class == "StarterPlayer" {
        // StarterPlayer has two special members with their own classes.

        if name == "StarterPlayerScripts" || name == "StarterCharacterScripts" {
            return Some(ustr(name));
        }
    } else if parent_class == "Workspace" {
        // Workspace has a special Terrain class inside it
        if name == "Terrain" {
            return Some(ustr(name));
        }
    }

    None
}

// #[cfg(feature = "broken-tests")]
#[cfg(test)]
mod test {
    use super::*;

    use memofs::{InMemoryFs, VfsSnapshot};

    #[ignore = "Functionality moved to root snapshot middleware"]
    #[test]
    fn project_from_folder() {
        let _ = env_logger::try_init();

        let mut imfs = InMemoryFs::new();
        imfs.load_snapshot(
            "/foo",
            VfsSnapshot::dir([(
                "default.project.json",
                VfsSnapshot::file(
                    r#"
                    {
                        "name": "indirect-project",
                        "tree": {
                            "$className": "Folder"
                        }
                    }
                "#,
                ),
            )]),
        )
        .unwrap();

        let vfs = Vfs::new(imfs);

        let instance_snapshot = snapshot_project(
            &InstanceContext::default(),
            &vfs,
            Path::new("/foo"),
            "NOT_IN_SNAPSHOT",
        )
        .expect("snapshot error")
        .expect("snapshot returned no instances");

        insta::assert_yaml_snapshot!(instance_snapshot);
    }

    #[test]
    fn project_from_direct_file() {
        let _ = env_logger::try_init();

        let mut imfs = InMemoryFs::new();
        imfs.load_snapshot(
            "/foo",
            VfsSnapshot::dir([(
                "hello.project.json",
                VfsSnapshot::file(
                    r#"
                    {
                        "name": "direct-project",
                        "tree": {
                            "$className": "Model"
                        }
                    }
                "#,
                ),
            )]),
        )
        .unwrap();

        let vfs = Vfs::new(imfs);

        let instance_snapshot = snapshot_project(
            &InstanceContext::default(),
            &vfs,
            Path::new("/foo/hello.project.json"),
            "NOT_IN_SNAPSHOT",
        )
        .expect("snapshot error")
        .expect("snapshot returned no instances");

        insta::assert_yaml_snapshot!(instance_snapshot);
    }

    #[test]
    fn project_with_resolved_properties() {
        let _ = env_logger::try_init();

        let mut imfs = InMemoryFs::new();
        imfs.load_snapshot(
            "/foo.project.json",
            VfsSnapshot::file(
                r#"
                    {
                        "name": "resolved-properties",
                        "tree": {
                            "$className": "StringValue",
                            "$properties": {
                                "Value": {
                                    "String": "Hello, world!"
                                }
                            }
                        }
                    }
                "#,
            ),
        )
        .unwrap();

        let vfs = Vfs::new(imfs);

        let instance_snapshot = snapshot_project(
            &InstanceContext::default(),
            &vfs,
            Path::new("/foo.project.json"),
            "NOT_IN_SNAPSHOT",
        )
        .expect("snapshot error")
        .expect("snapshot returned no instances");

        insta::assert_yaml_snapshot!(instance_snapshot);
    }

    #[test]
    fn project_with_unresolved_properties() {
        let _ = env_logger::try_init();

        let mut imfs = InMemoryFs::new();
        imfs.load_snapshot(
            "/foo.project.json",
            VfsSnapshot::file(
                r#"
                    {
                        "name": "unresolved-properties",
                        "tree": {
                            "$className": "StringValue",
                            "$properties": {
                                "Value": "Hi!"
                            }
                        }
                    }
                "#,
            ),
        )
        .unwrap();

        let vfs = Vfs::new(imfs);

        let instance_snapshot = snapshot_project(
            &InstanceContext::default(),
            &vfs,
            Path::new("/foo.project.json"),
            "NOT_IN_SNAPSHOT",
        )
        .expect("snapshot error")
        .expect("snapshot returned no instances");

        insta::assert_yaml_snapshot!(instance_snapshot);
    }

    #[test]
    fn project_with_children() {
        let _ = env_logger::try_init();

        let mut imfs = InMemoryFs::new();
        imfs.load_snapshot(
            "/foo.project.json",
            VfsSnapshot::file(
                r#"
                    {
                        "name": "children",
                        "tree": {
                            "$className": "Folder",

                            "Child": {
                                "$className": "Model"
                            }
                        }
                    }
                "#,
            ),
        )
        .unwrap();

        let vfs = Vfs::new(imfs);

        let instance_snapshot = snapshot_project(
            &InstanceContext::default(),
            &vfs,
            Path::new("/foo.project.json"),
            "NOT_IN_SNAPSHOT",
        )
        .expect("snapshot error")
        .expect("snapshot returned no instances");

        insta::assert_yaml_snapshot!(instance_snapshot);
    }

    #[test]
    fn project_with_path_to_txt() {
        let _ = env_logger::try_init();

        let mut imfs = InMemoryFs::new();
        imfs.load_snapshot(
            "/foo",
            VfsSnapshot::dir([
                (
                    "default.project.json",
                    VfsSnapshot::file(
                        r#"
                    {
                        "name": "path-project",
                        "tree": {
                            "$path": "other.txt"
                        }
                    }
                "#,
                    ),
                ),
                ("other.txt", VfsSnapshot::file("Hello, world!")),
            ]),
        )
        .unwrap();

        let vfs = Vfs::new(imfs);

        let instance_snapshot = snapshot_project(
            &InstanceContext::default(),
            &vfs,
            Path::new("/foo/default.project.json"),
            "NOT_IN_SNAPSHOT",
        )
        .expect("snapshot error")
        .expect("snapshot returned no instances");

        insta::assert_yaml_snapshot!(instance_snapshot);
    }

    #[test]
    fn project_with_path_to_project() {
        let _ = env_logger::try_init();

        let mut imfs = InMemoryFs::new();
        imfs.load_snapshot(
            "/foo",
            VfsSnapshot::dir([
                (
                    "default.project.json",
                    VfsSnapshot::file(
                        r#"
                    {
                        "name": "path-project",
                        "tree": {
                            "$path": "other.project.json"
                        }
                    }
                "#,
                    ),
                ),
                (
                    "other.project.json",
                    VfsSnapshot::file(
                        r#"
                    {
                        "name": "other-project",
                        "tree": {
                            "$className": "Model"
                        }
                    }
                "#,
                    ),
                ),
            ]),
        )
        .unwrap();

        let vfs = Vfs::new(imfs);

        let instance_snapshot = snapshot_project(
            &InstanceContext::default(),
            &vfs,
            Path::new("/foo/default.project.json"),
            "NOT_IN_SNAPSHOT",
        )
        .expect("snapshot error")
        .expect("snapshot returned no instances");

        insta::assert_yaml_snapshot!(instance_snapshot);
    }

    #[test]
    fn project_with_path_to_project_with_children() {
        let _ = env_logger::try_init();

        let mut imfs = InMemoryFs::new();
        imfs.load_snapshot(
            "/foo",
            VfsSnapshot::dir([
                (
                    "default.project.json",
                    VfsSnapshot::file(
                        r#"
                    {
                        "name": "path-child-project",
                        "tree": {
                            "$path": "other.project.json"
                        }
                    }
                "#,
                    ),
                ),
                (
                    "other.project.json",
                    VfsSnapshot::file(
                        r#"
                    {
                        "name": "other-project",
                        "tree": {
                            "$className": "Folder",

                            "SomeChild": {
                                "$className": "Model"
                            }
                        }
                    }
                "#,
                    ),
                ),
            ]),
        )
        .unwrap();

        let vfs = Vfs::new(imfs);

        let instance_snapshot = snapshot_project(
            &InstanceContext::default(),
            &vfs,
            Path::new("/foo/default.project.json"),
            "NOT_IN_SNAPSHOT",
        )
        .expect("snapshot error")
        .expect("snapshot returned no instances");

        insta::assert_yaml_snapshot!(instance_snapshot);
    }

    /// Ensures that if a property is defined both in the resulting instance
    /// from $path and also in $properties, that the $properties value takes
    /// precedence.
    #[test]
    fn project_path_property_overrides() {
        let _ = env_logger::try_init();

        let mut imfs = InMemoryFs::new();
        imfs.load_snapshot(
            "/foo",
            VfsSnapshot::dir([
                (
                    "default.project.json",
                    VfsSnapshot::file(
                        r#"
                    {
                        "name": "path-property-override",
                        "tree": {
                            "$path": "other.project.json",
                            "$properties": {
                                "Value": "Changed"
                            }
                        }
                    }
                "#,
                    ),
                ),
                (
                    "other.project.json",
                    VfsSnapshot::file(
                        r#"
                    {
                        "name": "other-project",
                        "tree": {
                            "$className": "StringValue",
                            "$properties": {
                                "Value": "Original"
                            }
                        }
                    }
                "#,
                    ),
                ),
            ]),
        )
        .unwrap();

        let vfs = Vfs::new(imfs);

        let instance_snapshot = snapshot_project(
            &InstanceContext::default(),
            &vfs,
            Path::new("/foo/default.project.json"),
            "NOT_IN_SNAPSHOT",
        )
        .expect("snapshot error")
        .expect("snapshot returned no instances");

        insta::assert_yaml_snapshot!(instance_snapshot);
    }

    #[test]
    fn no_name_project() {
        let _ = env_logger::try_init();

        let mut imfs = InMemoryFs::new();
        imfs.load_snapshot(
            "/foo",
            VfsSnapshot::dir([(
                "default.project.json",
                VfsSnapshot::file(
                    r#"
                    {
                        "tree": {
                            "$className": "Model"
                        }
                    }
                "#,
                ),
            )]),
        )
        .unwrap();

        let vfs = Vfs::new(imfs);

        let instance_snapshot = snapshot_project(
            &InstanceContext::default(),
            &vfs,
            Path::new("/foo/default.project.json"),
            "no_name_project",
        )
        .expect("snapshot error")
        .expect("snapshot returned no instances");

        insta::assert_yaml_snapshot!(instance_snapshot);
    }
}
