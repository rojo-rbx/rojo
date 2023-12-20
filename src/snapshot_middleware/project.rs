use std::{borrow::Cow, collections::HashMap, path::Path};

use anyhow::{bail, Context};
use memofs::Vfs;
use rbx_dom_weak::types::{Attributes, Ref};
use rbx_reflection::ClassTag;

use crate::{
    project::{PathNode, Project, ProjectNode},
    resolution::UnresolvedValue,
    snapshot::{
        InstanceContext, InstanceMetadata, InstanceSnapshot, InstigatingSource, PathIgnoreRule,
        SyncRule,
    },
    syncback::{FsSnapshot, SyncbackReturn, SyncbackSnapshot},
};

use super::{emit_legacy_scripts_default, snapshot_from_vfs};

pub fn snapshot_project(
    context: &InstanceContext,
    vfs: &Vfs,
    path: &Path,
) -> anyhow::Result<Option<InstanceSnapshot>> {
    let project = Project::load_from_slice(&vfs.read(path)?, path)
        .with_context(|| format!("File was not a valid Rojo project: {}", path.display()))?;

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

    match snapshot_project_node(&context, path, &project.name, &project.tree, vfs, None)? {
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

    let class_name_from_project = node
        .class_name
        .as_ref()
        .map(|name| Cow::Owned(name.clone()));
    let mut class_name_from_path = None;

    let name = Cow::Owned(instance_name.to_owned());
    let mut properties = HashMap::new();
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
        class_name_from_project,
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

        properties.insert(key.clone(), value);
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

    metadata.instigating_source = Some(InstigatingSource::ProjectNode {
        path: project_path.to_path_buf(),
        name: instance_name.to_string(),
        node: node.clone(),
        parent_class: parent_class.map(|name| name.to_owned()),
    });

    Ok(Some(InstanceSnapshot {
        snapshot_id: Ref::none(),
        name,
        class_name,
        properties,
        children,
        metadata,
    }))
}

pub fn syncback_project<'new, 'old>(
    snapshot: &SyncbackSnapshot<'new, 'old>,
) -> anyhow::Result<SyncbackReturn<'new, 'old>> {
    let old_inst = snapshot
        .old_inst()
        .context("project middleware shouldn't be used to make new files")?;
    // This can never be None.
    let source = old_inst.metadata().instigating_source.as_ref().unwrap();

    // We need to build a 'new' project and serialize it using an FsSnapshot.
    // It's convenient to start with the old one though, since it means we have
    // a thing to iterate through.
    let mut project =
        Project::load_from_slice(&snapshot.vfs().read(source.path()).unwrap(), source.path())
            .context("could not syncback project due to fs error")?;

    let base_path = source.path().parent().unwrap();

    let mut children = Vec::new();
    let mut removed_children = Vec::new();

    // Projects are special. We won't be adding or removing things from them,
    // so we'll simply match Instances on a per-node basis and rebuild the tree
    // with the new instance's data. This matching will be done by class and name
    // to simplify things.
    let mut nodes = vec![(&mut project.tree, snapshot.new_inst(), old_inst)];

    // A map of referents from the new tree to the Path that created it,
    // if it exists. This is a roundabout way to locate the parents of
    // Instances.
    let mut ref_to_node = HashMap::new();

    while let Some((node, new_inst, old_inst)) = nodes.pop() {
        ref_to_node.insert(new_inst.referent(), node.path.as_ref());

        let mut old_child_map = HashMap::with_capacity(old_inst.children().len());
        for child_ref in old_inst.children() {
            let child = snapshot.get_old_instance(*child_ref).unwrap();
            old_child_map.insert(child.name(), child);
        }
        let mut new_child_map = HashMap::with_capacity(new_inst.children().len());
        for child_ref in new_inst.children() {
            let child = snapshot.get_new_instance(*child_ref).unwrap();
            new_child_map.insert(child.name.as_str(), child);
        }

        for (child_name, child_node) in &mut node.children {
            if let Some(new_child) = new_child_map.get(child_name.as_str()) {
                if let Some(old_child) = old_child_map.get(child_name.as_str()) {
                    if &new_child.class != old_child.class_name() {
                        anyhow::bail!("Cannot change the class of items in a project");
                    }
                    for (name, value) in &new_child.properties {
                        if child_node.properties.contains_key(name) {
                            child_node
                                .properties
                                .insert(name.clone(), UnresolvedValue::from(value.clone()));
                        }
                    }
                    nodes.push((child_node, new_child, *old_child));
                    new_child_map.remove(child_name.as_str());
                    old_child_map.remove(child_name.as_str());
                }
            } else {
                anyhow::bail!("Cannot add or remove children from a project")
            }
        }

        // From this point, both maps contain only children of the current
        // instance that aren't in the project. So, we just do some quick and
        // dirty matching to identify children that were added and removed.
        for (new_name, new_child) in new_child_map {
            let parent_path = match ref_to_node.get(&new_child.parent()) {
                Some(Some(path)) => base_path.join(path.path()),
                Some(None) => {
                    continue;
                }
                None => {
                    continue;
                }
            };
            if let Some(old_inst) = old_child_map.get(new_name) {
                // All children are descendants of a node of a project
                // So we really just need to track which one is which.
                children.push(SyncbackSnapshot {
                    data: snapshot.data.clone(),
                    old: Some(old_inst.id()),
                    new: new_child.referent(),
                    parent_path,
                    name: new_name.to_string(),
                });
                old_child_map.remove(new_name);
            } else {
                // it's new
                children.push(SyncbackSnapshot {
                    data: snapshot.data.clone(),
                    old: None,
                    new: new_child.referent(),
                    parent_path,
                    name: new_name.to_string(),
                });
            }
        }
        removed_children.extend(old_child_map.into_values());
    }

    // We don't need to validate any file names for the FsSnapshot
    // because projects can't ever be made by syncback, so they must
    // already exist on the file system
    Ok(SyncbackReturn {
        inst_snapshot: InstanceSnapshot::from_instance(snapshot.new_inst()),
        fs_snapshot: FsSnapshot::new().with_file(
            &project.file_location,
            serde_json::to_vec_pretty(&project).context("failed to serialize new project")?,
        ),
        children,
        removed_children,
    })
}

fn infer_class_name(name: &str, parent_class: Option<&str>) -> Option<Cow<'static, str>> {
    // If className wasn't defined from another source, we may be able
    // to infer one.

    let parent_class = parent_class?;

    if parent_class == "DataModel" {
        // Members of DataModel with names that match known services are
        // probably supposed to be those services.

        let descriptor = rbx_reflection_database::get().classes.get(name)?;

        if descriptor.tags.contains(&ClassTag::Service) {
            return Some(Cow::Owned(name.to_owned()));
        }
    } else if parent_class == "StarterPlayer" {
        // StarterPlayer has two special members with their own classes.

        if name == "StarterPlayerScripts" || name == "StarterCharacterScripts" {
            return Some(Cow::Owned(name.to_owned()));
        }
    } else if parent_class == "Workspace" {
        // Workspace has a special Terrain class inside it
        if name == "Terrain" {
            return Some(Cow::Owned(name.to_owned()));
        }
    }

    None
}

// #[cfg(feature = "broken-tests")]
#[cfg(test)]
mod test {
    use super::*;

    use maplit::hashmap;
    use memofs::{InMemoryFs, VfsSnapshot};

    #[ignore = "Functionality moved to root snapshot middleware"]
    #[test]
    fn project_from_folder() {
        let _ = env_logger::try_init();

        let mut imfs = InMemoryFs::new();
        imfs.load_snapshot(
            "/foo",
            VfsSnapshot::dir(hashmap! {
                "default.project.json" => VfsSnapshot::file(r#"
                    {
                        "name": "indirect-project",
                        "tree": {
                            "$className": "Folder"
                        }
                    }
                "#),
            }),
        )
        .unwrap();

        let mut vfs = Vfs::new(imfs);

        let instance_snapshot =
            snapshot_project(&InstanceContext::default(), &mut vfs, Path::new("/foo"))
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
            VfsSnapshot::dir(hashmap! {
                "hello.project.json" => VfsSnapshot::file(r#"
                    {
                        "name": "direct-project",
                        "tree": {
                            "$className": "Model"
                        }
                    }
                "#),
            }),
        )
        .unwrap();

        let mut vfs = Vfs::new(imfs);

        let instance_snapshot = snapshot_project(
            &InstanceContext::default(),
            &mut vfs,
            Path::new("/foo/hello.project.json"),
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

        let mut vfs = Vfs::new(imfs);

        let instance_snapshot = snapshot_project(
            &InstanceContext::default(),
            &mut vfs,
            Path::new("/foo.project.json"),
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

        let mut vfs = Vfs::new(imfs);

        let instance_snapshot = snapshot_project(
            &InstanceContext::default(),
            &mut vfs,
            Path::new("/foo.project.json"),
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

        let mut vfs = Vfs::new(imfs);

        let instance_snapshot = snapshot_project(
            &InstanceContext::default(),
            &mut vfs,
            Path::new("/foo.project.json"),
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
            VfsSnapshot::dir(hashmap! {
                "default.project.json" => VfsSnapshot::file(r#"
                    {
                        "name": "path-project",
                        "tree": {
                            "$path": "other.txt"
                        }
                    }
                "#),
                "other.txt" => VfsSnapshot::file("Hello, world!"),
            }),
        )
        .unwrap();

        let mut vfs = Vfs::new(imfs);

        let instance_snapshot = snapshot_project(
            &InstanceContext::default(),
            &mut vfs,
            Path::new("/foo/default.project.json"),
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
            VfsSnapshot::dir(hashmap! {
                "default.project.json" => VfsSnapshot::file(r#"
                    {
                        "name": "path-project",
                        "tree": {
                            "$path": "other.project.json"
                        }
                    }
                "#),
                "other.project.json" => VfsSnapshot::file(r#"
                    {
                        "name": "other-project",
                        "tree": {
                            "$className": "Model"
                        }
                    }
                "#),
            }),
        )
        .unwrap();

        let mut vfs = Vfs::new(imfs);

        let instance_snapshot = snapshot_project(
            &InstanceContext::default(),
            &mut vfs,
            Path::new("/foo/default.project.json"),
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
            VfsSnapshot::dir(hashmap! {
                "default.project.json" => VfsSnapshot::file(r#"
                    {
                        "name": "path-child-project",
                        "tree": {
                            "$path": "other.project.json"
                        }
                    }
                "#),
                "other.project.json" => VfsSnapshot::file(r#"
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
            }),
        )
        .unwrap();

        let mut vfs = Vfs::new(imfs);

        let instance_snapshot = snapshot_project(
            &InstanceContext::default(),
            &mut vfs,
            Path::new("/foo/default.project.json"),
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
            VfsSnapshot::dir(hashmap! {
                "default.project.json" => VfsSnapshot::file(r#"
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
                "other.project.json" => VfsSnapshot::file(r#"
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
            }),
        )
        .unwrap();

        let mut vfs = Vfs::new(imfs);

        let instance_snapshot = snapshot_project(
            &InstanceContext::default(),
            &mut vfs,
            Path::new("/foo/default.project.json"),
        )
        .expect("snapshot error")
        .expect("snapshot returned no instances");

        insta::assert_yaml_snapshot!(instance_snapshot);
    }
}
