use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

use anyhow::Context;
use maplit::hashmap;
use memofs::{IoResultExt, Vfs};
use rbx_dom_weak::types::Variant;
use serde::{Deserialize, Serialize};

use crate::{
    snapshot::{InstanceContext, InstanceMetadata, InstanceSnapshot},
    syncback::{FsSnapshot, SyncbackReturn, SyncbackSnapshot},
};

use super::{
    dir::{dir_meta, snapshot_dir_no_meta, syncback_dir_no_meta},
    meta_file::{AdjacentMetadata, DirectoryMetadata},
    PathExt as _,
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

pub fn syncback_csv<'sync>(
    snapshot: &SyncbackSnapshot<'sync>,
) -> anyhow::Result<SyncbackReturn<'sync>> {
    let new_inst = snapshot.new_inst();

    let contents = if let Some(Variant::String(content)) = new_inst.properties.get("Contents") {
        content.as_str()
    } else {
        anyhow::bail!("LocalizationTables must have a `Contents` property that is a String")
    };
    let mut fs_snapshot = FsSnapshot::new();
    fs_snapshot.add_file(&snapshot.path, localization_to_csv(contents)?);

    let meta = AdjacentMetadata::from_syncback_snapshot(snapshot, snapshot.path.clone())?;
    if let Some(mut meta) = meta {
        meta.properties.remove("Contents");

        if !meta.is_empty() {
            let parent = snapshot.path.parent_err()?;
            fs_snapshot.add_file(
                parent.join(format!("{}.meta.json", new_inst.name)),
                serde_json::to_vec_pretty(&meta).context("cannot serialize metadata")?,
            )
        }
    }

    Ok(SyncbackReturn {
        inst_snapshot: InstanceSnapshot::from_instance(new_inst),
        fs_snapshot,
        children: Vec::new(),
        removed_children: Vec::new(),
    })
}

pub fn syncback_csv_init<'sync>(
    snapshot: &SyncbackSnapshot<'sync>,
) -> anyhow::Result<SyncbackReturn<'sync>> {
    let new_inst = snapshot.new_inst();

    let contents = if let Some(Variant::String(content)) = new_inst.properties.get("Contents") {
        content.as_str()
    } else {
        anyhow::bail!("LocalizationTables must have a `Contents` property that is a String")
    };

    let mut dir_syncback = syncback_dir_no_meta(snapshot)?;
    dir_syncback.fs_snapshot.add_file(
        &snapshot.path.join("init.csv"),
        localization_to_csv(contents)?,
    );

    let meta = DirectoryMetadata::from_syncback_snapshot(snapshot, snapshot.path.clone())?;
    if let Some(mut meta) = meta {
        meta.properties.remove("Contents");
        if !meta.is_empty() {
            dir_syncback.fs_snapshot.add_file(
                snapshot.path.join("init.meta.json"),
                serde_json::to_vec_pretty(&meta)
                    .context("could not serialize new init.meta.json")?,
            );
        }
    }

    Ok(SyncbackReturn {
        inst_snapshot: InstanceSnapshot::from_instance(new_inst),
        ..dir_syncback
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
    key: Option<&'a str>,

    #[serde(skip_serializing_if = "Option::is_none")]
    context: Option<&'a str>,

    #[serde(skip_serializing_if = "Option::is_none")]
    examples: Option<&'a str>,

    #[serde(skip_serializing_if = "Option::is_none")]
    source: Option<&'a str>,

    // We use a BTreeMap here to get deterministic output order.
    values: BTreeMap<&'a str, &'a str>,
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
                "Key" => entry.key = Some(value),
                "Source" => entry.source = Some(value),
                "Context" => entry.context = Some(value),
                "Example" => entry.examples = Some(value),
                _ => {
                    entry.values.insert(header, value);
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
        record.push(entry.key.unwrap_or_default());
        record.push(entry.source.unwrap_or_default());
        record.push(entry.context.unwrap_or_default());
        record.push(entry.examples.unwrap_or_default());

        let values = &entry.values;
        for header in &extra_headers {
            record.push(values.get(*header).copied().unwrap_or_default());
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
