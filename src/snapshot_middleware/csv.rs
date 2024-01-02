use std::{
    borrow::Cow,
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

use anyhow::Context;
use maplit::hashmap;
use memofs::{IoResultExt, Vfs};
use rbx_dom_weak::types::Variant;
use serde::{Deserialize, Serialize};

use crate::{
    resolution::UnresolvedValue,
    snapshot::{InstanceContext, InstanceMetadata, InstanceSnapshot},
    syncback::{FsSnapshot, SyncbackReturn, SyncbackSnapshot},
};

use super::{
    dir::{dir_meta, snapshot_dir_no_meta, syncback_dir_no_meta},
    meta_file::{file_meta, AdjacentMetadata},
    DirectoryMetadata,
};

pub fn snapshot_csv(
    _context: &InstanceContext,
    vfs: &Vfs,
    path: &Path,
    name: &str,
) -> anyhow::Result<Option<InstanceSnapshot>> {
    let meta_path = path.with_file_name(format!("{}.meta.json", name));
    let contents = vfs.read(path)?;

    let table_contents = convert_localization_csv(&contents).with_context(|| {
        format!(
            "File was not a valid LocalizationTable CSV file: {}",
            path.display()
        )
    })?;

    let mut snapshot = InstanceSnapshot::new()
        .name(name)
        .class_name("LocalizationTable")
        .properties(hashmap! {
            "Contents".to_owned() => table_contents.into(),
        })
        .metadata(
            InstanceMetadata::new()
                .instigating_source(path)
                .relevant_paths(vec![path.to_path_buf(), meta_path.clone()]),
        );

    if let Some(meta_contents) = vfs.read(&meta_path).with_not_found()? {
        let mut metadata = AdjacentMetadata::from_slice(&meta_contents, meta_path)?;
        metadata.apply_all(&mut snapshot)?;
    }

    Ok(Some(snapshot))
}

/// Attempts to snapshot an 'init' csv contained inside of a folder with
/// the given name.
///
/// csv named `init.csv`
/// their parents, which acts similarly to `__init__.py` from the Python world.
pub fn snapshot_csv_init(
    context: &InstanceContext,
    vfs: &Vfs,
    init_path: &Path,
    name: &str,
) -> anyhow::Result<Option<InstanceSnapshot>> {
    let folder_path = init_path.parent().unwrap();
    let dir_snapshot = snapshot_dir_no_meta(context, vfs, folder_path, name)?.unwrap();

    if dir_snapshot.class_name != "Folder" {
        anyhow::bail!(
            "init.csv can only be used if the instance produced by \
             the containing directory would be a Folder.\n\
             \n\
             The directory {} turned into an instance of class {}.",
            folder_path.display(),
            dir_snapshot.class_name
        );
    }

    let mut init_snapshot = snapshot_csv(context, vfs, init_path, &dir_snapshot.name)?.unwrap();

    init_snapshot.children = dir_snapshot.children;
    init_snapshot.metadata = dir_snapshot.metadata;
    init_snapshot
        .metadata
        .relevant_paths
        .push(init_path.to_owned());

    if let Some(mut meta) = dir_meta(vfs, folder_path)? {
        meta.apply_all(&mut init_snapshot)?;
    }

    Ok(Some(init_snapshot))
}

pub fn syncback_csv<'new, 'old>(
    snapshot: &SyncbackSnapshot<'new, 'old>,
) -> anyhow::Result<SyncbackReturn<'new, 'old>> {
    let new_inst = snapshot.new_inst();

    let mut path = snapshot.parent_path.join(&snapshot.name);
    path.set_extension("csv");

    let contents = if let Some(Variant::String(content)) = new_inst.properties.get("Contents") {
        content.as_str()
    } else {
        anyhow::bail!("LocalizationTables must have a `Contents` property that is a String")
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
        if name == "Contents" {
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

    // TODO tags don't work, why?
    let mut fs_snapshot = FsSnapshot::new();
    fs_snapshot.push_file(path, localization_to_csv(contents)?);
    if !meta.is_empty() {
        fs_snapshot.push_file(
            &meta.path,
            serde_json::to_vec_pretty(&meta).context("failed to reserialize metadata")?,
        )
    }

    Ok(SyncbackReturn {
        inst_snapshot: InstanceSnapshot::from_instance(new_inst),
        fs_snapshot,
        children: Vec::new(),
        removed_children: Vec::new(),
    })
}

pub fn syncback_csv_init<'new, 'old>(
    snapshot: &SyncbackSnapshot<'new, 'old>,
) -> anyhow::Result<SyncbackReturn<'new, 'old>> {
    let new_inst = snapshot.new_inst();

    let mut path = snapshot.parent_path.join(&snapshot.name);
    path.push("init.csv");

    let contents = if let Some(Variant::String(content)) = new_inst.properties.get("Contents") {
        content.as_str()
    } else {
        anyhow::bail!("LocalizationTables must have a `Contents` property that is a String")
    };

    let mut dir_syncback = syncback_dir_no_meta(snapshot)?;
    let mut meta = if let Some(dir) = dir_meta(snapshot.vfs(), &path)? {
        dir
    } else {
        DirectoryMetadata {
            ignore_unknown_instances: None,
            class_name: None,
            properties: BTreeMap::new(),
            attributes: BTreeMap::new(),
            path: snapshot
                .parent_path
                .join(&snapshot.name)
                .join("init.meta.json"),
        }
    };
    for (name, value) in snapshot.get_filtered_properties() {
        if name == "Contents" {
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

    let mut fs_snapshot = std::mem::take(&mut dir_syncback.fs_snapshot);
    fs_snapshot.push_file(&path, localization_to_csv(contents)?);
    if !meta.is_empty() {
        fs_snapshot.push_file(
            &meta.path,
            serde_json::to_vec_pretty(&meta).context("could not serialize new init.meta.json")?,
        );
    }

    Ok(SyncbackReturn {
        inst_snapshot: InstanceSnapshot::from_instance(new_inst),
        fs_snapshot,
        children: dir_syncback.children,
        removed_children: dir_syncback.removed_children,
    })
}

/// Struct that holds any valid row from a Roblox CSV translation table.
///
/// We manually deserialize into this table from CSV, but let serde_json handle
/// serialization.
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LocalizationEntry<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    key: Option<Cow<'a, str>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    context: Option<Cow<'a, str>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    example: Option<Cow<'a, str>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    source: Option<Cow<'a, str>>,

    // We use a BTreeMap here to get deterministic output order.
    values: BTreeMap<Cow<'a, str>, Cow<'a, str>>,
}

/// Normally, we'd be able to let the csv crate construct our struct for us.
///
/// However, because of a limitation with Serde's 'flatten' feature, it's not
/// possible presently to losslessly collect extra string values while using
/// csv+Serde.
///
/// https://github.com/BurntSushi/rust-csv/issues/151
///
/// This function operates in one step in order to minimize data-copying.
fn convert_localization_csv(contents: &[u8]) -> Result<String, csv::Error> {
    let mut reader = csv::Reader::from_reader(contents);

    let headers = reader.headers()?.clone();

    let mut records = Vec::new();

    for record in reader.into_records() {
        records.push(record?);
    }

    let mut entries = Vec::new();

    for record in &records {
        let mut entry = LocalizationEntry::default();

        for (header, value) in headers.iter().zip(record.into_iter()) {
            if header.is_empty() || value.is_empty() {
                continue;
            }

            match header {
                "Key" => entry.key = Some(Cow::Borrowed(value)),
                "Source" => entry.source = Some(Cow::Borrowed(value)),
                "Context" => entry.context = Some(Cow::Borrowed(value)),
                "Example" => entry.example = Some(Cow::Borrowed(value)),
                _ => {
                    entry
                        .values
                        .insert(Cow::Borrowed(value), Cow::Borrowed(value));
                }
            }
        }

        if entry.key.is_none() && entry.source.is_none() {
            continue;
        }

        entries.push(entry);
    }

    let encoded =
        serde_json::to_string(&entries).expect("Could not encode JSON for localization table");

    Ok(encoded)
}

/// Takes a localization table (as a string) and converts it into a CSV file.
///
/// The CSV file is ordered, so it should be deterministic.
fn localization_to_csv(csv_contents: &str) -> anyhow::Result<Vec<u8>> {
    let mut out = Vec::new();
    let mut writer = csv::Writer::from_writer(&mut out);

    let mut csv: Vec<LocalizationEntry> =
        serde_json::from_str(csv_contents).context("cannot decode JSON from localization table")?;

    // TODO sort this better
    csv.sort_unstable_by(|a, b| a.source.partial_cmp(&b.source).unwrap());

    let mut headers = vec!["Key", "Source", "Context", "Example"];
    // We want both order and a lack of duplicates, so we use a BTreeSet.
    let mut extra_headers = BTreeSet::new();
    for entry in &csv {
        for lang in entry.values.keys() {
            extra_headers.insert(lang.as_ref());
        }
    }
    headers.extend(extra_headers.iter());

    writer
        .write_record(&headers)
        .context("could not write headers for localization table")?;

    let mut record: Vec<&str> = Vec::with_capacity(headers.len());
    for entry in &csv {
        record.push(entry.key.as_ref().unwrap_or(&Cow::Borrowed("")));
        record.push(entry.source.as_ref().unwrap_or(&Cow::Borrowed("")));
        record.push(entry.context.as_ref().unwrap_or(&Cow::Borrowed("")));
        record.push(entry.example.as_ref().unwrap_or(&Cow::Borrowed("")));

        let values = &entry.values;
        for header in &extra_headers {
            record.push(values.get(*header).unwrap_or(&Cow::Borrowed("")));
        }

        writer
            .write_record(&record)
            .context("cannot write record for localization table")?;
        record.clear();
    }

    // We must drop `writer` here to regain access to `out`.
    drop(writer);

    Ok(out)
}

#[cfg(test)]
mod test {
    use super::*;

    use memofs::{InMemoryFs, VfsSnapshot};

    #[test]
    fn csv_from_vfs() {
        let mut imfs = InMemoryFs::new();
        imfs.load_snapshot(
            "/foo.csv",
            VfsSnapshot::file(
                r#"
Key,Source,Context,Example,es
Ack,Ack!,,An exclamation of despair,¡Ay!"#,
            ),
        )
        .unwrap();

        let mut vfs = Vfs::new(imfs);

        let instance_snapshot = snapshot_csv(
            &InstanceContext::default(),
            &mut vfs,
            Path::new("/foo.csv"),
            "foo",
        )
        .unwrap()
        .unwrap();

        insta::assert_yaml_snapshot!(instance_snapshot);
    }

    #[test]
    fn csv_with_meta() {
        let mut imfs = InMemoryFs::new();
        imfs.load_snapshot(
            "/foo.csv",
            VfsSnapshot::file(
                r#"
Key,Source,Context,Example,es
Ack,Ack!,,An exclamation of despair,¡Ay!"#,
            ),
        )
        .unwrap();
        imfs.load_snapshot(
            "/foo.meta.json",
            VfsSnapshot::file(r#"{ "ignoreUnknownInstances": true }"#),
        )
        .unwrap();

        let mut vfs = Vfs::new(imfs);

        let instance_snapshot = snapshot_csv(
            &InstanceContext::default(),
            &mut vfs,
            Path::new("/foo.csv"),
            "foo",
        )
        .unwrap()
        .unwrap();

        insta::assert_yaml_snapshot!(instance_snapshot);
    }
}
