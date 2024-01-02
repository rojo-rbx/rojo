use std::{collections::BTreeMap, path::Path, str};

use anyhow::Context;
use maplit::hashmap;
use memofs::{IoResultExt, Vfs};
use rbx_dom_weak::types::Variant;

use crate::{
    resolution::UnresolvedValue,
    snapshot::{InstanceContext, InstanceMetadata, InstanceSnapshot},
    syncback::{FsSnapshot, SyncbackReturn, SyncbackSnapshot},
};

use super::meta_file::{file_meta, AdjacentMetadata};

pub fn snapshot_txt(
    context: &InstanceContext,
    vfs: &Vfs,
    path: &Path,
    name: &str,
) -> anyhow::Result<Option<InstanceSnapshot>> {
    let contents = vfs.read(path)?;
    let contents_str = str::from_utf8(&contents)
        .with_context(|| format!("File was not valid UTF-8: {}", path.display()))?
        .to_owned();

    let properties = hashmap! {
        "Value".to_owned() => contents_str.into(),
    };

    let meta_path = path.with_file_name(format!("{}.meta.json", name));

    let mut snapshot = InstanceSnapshot::new()
        .name(name)
        .class_name("StringValue")
        .properties(properties)
        .metadata(
            InstanceMetadata::new()
                .instigating_source(path)
                .relevant_paths(vec![path.to_path_buf(), meta_path.clone()])
                .context(context),
        );

    if let Some(meta_contents) = vfs.read(&meta_path).with_not_found()? {
        let mut metadata = AdjacentMetadata::from_slice(&meta_contents, meta_path)?;
        metadata.apply_all(&mut snapshot)?;
    }

    Ok(Some(snapshot))
}

pub fn syncback_txt<'new, 'old>(
    snapshot: &SyncbackSnapshot<'new, 'old>,
) -> anyhow::Result<SyncbackReturn<'new, 'old>> {
    let new_inst = snapshot.new_inst();
    let mut path = snapshot.parent_path.join(&snapshot.name);
    path.set_extension("txt");

    let contents = if let Some(Variant::String(source)) = new_inst.properties.get("Value") {
        source.as_bytes().to_vec()
    } else {
        anyhow::bail!("StringValues must have a `Value` property that is a String");
    };

    let mut meta = if let Some(meta) = file_meta(snapshot.vfs(), &path, &snapshot.name)? {
        meta
    } else {
        AdjacentMetadata {
            ignore_unknown_instances: None,
            properties: BTreeMap::new(),
            attributes: BTreeMap::new(),
            path: path
                .with_file_name(&snapshot.name)
                .with_extension("meta.json"),
        }
    };
    for (name, value) in snapshot.get_filtered_properties() {
        if name == "Value" {
            continue;
        } else if name == "Attributes" || name == "AttributesSerialize" {
            if let Variant::Attributes(attrs) = value {
                meta.attributes.extend(attrs.iter().map(|(name, value)| {
                    (
                        name.to_string(),
                        UnresolvedValue::FullyQualified(value.clone()),
                    )
                }))
            } else {
                log::error!("Property {name} should be Attributes but is not");
            }
        } else {
            meta.properties.insert(
                name.to_string(),
                UnresolvedValue::from_variant(value.to_owned(), &new_inst.class, name),
            );
        }
    }

    Ok(SyncbackReturn {
        inst_snapshot: InstanceSnapshot::from_instance(new_inst),
        fs_snapshot: FsSnapshot::new().with_added_file(path, contents),
        children: Vec::new(),
        removed_children: Vec::new(),
    })
}

#[cfg(test)]
mod test {
    use super::*;

    use memofs::{InMemoryFs, VfsSnapshot};

    #[test]
    fn instance_from_vfs() {
        let mut imfs = InMemoryFs::new();
        imfs.load_snapshot("/foo.txt", VfsSnapshot::file("Hello there!"))
            .unwrap();

        let mut vfs = Vfs::new(imfs.clone());

        let instance_snapshot = snapshot_txt(
            &InstanceContext::default(),
            &mut vfs,
            Path::new("/foo.txt"),
            "foo",
        )
        .unwrap()
        .unwrap();

        insta::assert_yaml_snapshot!(instance_snapshot);
    }
}
