use std::{collections::BTreeMap, path::Path};

use maplit::hashmap;
use memofs::{IoResultExt, Vfs};
use rbx_dom_weak::RbxValue;
use serde::Serialize;

use crate::snapshot::{InstanceContext, InstanceMetadata, InstanceSnapshot};

use super::{
    error::SnapshotError, meta_file::AdjacentMetadata, middleware::SnapshotInstanceResult,
};

pub fn snapshot_csv(
    _context: &InstanceContext,
    vfs: &Vfs,
    path: &Path,
    instance_name: &str,
) -> SnapshotInstanceResult {
    let meta_path = path.with_file_name(format!("{}.meta.json", instance_name));
    let contents = vfs.read(path)?;

    let table_contents = convert_localization_csv(&contents)
        .map_err(|source| SnapshotError::malformed_l10n_csv(source, path))?;

    let mut snapshot = InstanceSnapshot::new()
        .name(instance_name)
        .class_name("LocalizationTable")
        .properties(hashmap! {
            "Contents".to_owned() => RbxValue::String {
                value: table_contents,
            },
        })
        .metadata(
            InstanceMetadata::new()
                .instigating_source(path)
                .relevant_paths(vec![path.to_path_buf(), meta_path.clone()]),
        );

    if let Some(meta_contents) = vfs.read(&meta_path).with_not_found()? {
        let mut metadata = AdjacentMetadata::from_slice(&meta_contents, &meta_path)?;
        metadata.apply_all(&mut snapshot);
    }

    Ok(Some(snapshot))
}

/// Struct that holds any valid row from a Roblox CSV translation table.
///
/// We manually deserialize into this table from CSV, but let serde_json handle
/// serialization.
#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
struct LocalizationEntry<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    key: Option<&'a str>,

    #[serde(skip_serializing_if = "Option::is_none")]
    context: Option<&'a str>,

    #[serde(skip_serializing_if = "Option::is_none")]
    example: Option<&'a str>,

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
                "Example" => entry.example = Some(value),
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
