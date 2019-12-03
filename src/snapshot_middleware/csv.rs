use std::collections::BTreeMap;

use maplit::hashmap;
use rbx_dom_weak::RbxValue;
use serde::Serialize;

use crate::{
    snapshot::{InstanceContext, InstanceMetadata, InstanceSnapshot},
    vfs::{FsResultExt, Vfs, VfsEntry, VfsFetcher},
};

use super::{
    meta_file::AdjacentMetadata,
    middleware::{SnapshotInstanceResult, SnapshotMiddleware},
    util::match_file_name,
};

pub struct SnapshotCsv;

impl SnapshotMiddleware for SnapshotCsv {
    fn from_vfs<F: VfsFetcher>(
        _context: &mut InstanceContext,
        vfs: &Vfs<F>,
        entry: &VfsEntry,
    ) -> SnapshotInstanceResult {
        if entry.is_directory() {
            return Ok(None);
        }

        let instance_name = match match_file_name(entry.path(), ".csv") {
            Some(name) => name,
            None => return Ok(None),
        };

        let meta_path = entry
            .path()
            .with_file_name(format!("{}.meta.json", instance_name));

        let table_contents = convert_localization_csv(&entry.contents(vfs)?);

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
                    .instigating_source(entry.path())
                    .relevant_paths(vec![entry.path().to_path_buf(), meta_path.clone()]),
            );

        if let Some(meta_entry) = vfs.get(meta_path).with_not_found()? {
            let meta_contents = meta_entry.contents(vfs)?;
            let mut metadata = AdjacentMetadata::from_slice(&meta_contents);
            metadata.apply_all(&mut snapshot);
        }

        Ok(Some(snapshot))
    }
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
fn convert_localization_csv(contents: &[u8]) -> String {
    let mut reader = csv::Reader::from_reader(contents);

    let headers = reader.headers().expect("TODO: Handle csv errors").clone();

    let mut records = Vec::new();

    for record in reader.into_records() {
        let record = record.expect("TODO: Handle csv errors");

        records.push(record);
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

    serde_json::to_string(&entries).expect("Could not encode JSON for localization table")
}

#[cfg(test)]
mod test {
    use super::*;

    use crate::vfs::{NoopFetcher, VfsDebug, VfsSnapshot};
    use insta::assert_yaml_snapshot;

    #[test]
    fn csv_from_vfs() {
        let mut vfs = Vfs::new(NoopFetcher);
        let file = VfsSnapshot::file(
            r#"
Key,Source,Context,Example,es
Ack,Ack!,,An exclamation of despair,¡Ay!"#,
        );

        vfs.debug_load_snapshot("/foo.csv", file);

        let entry = vfs.get("/foo.csv").unwrap();
        let instance_snapshot =
            SnapshotCsv::from_vfs(&mut InstanceContext::default(), &mut vfs, &entry)
                .unwrap()
                .unwrap();

        assert_yaml_snapshot!(instance_snapshot);
    }

    #[test]
    fn csv_with_meta() {
        let mut vfs = Vfs::new(NoopFetcher);
        let file = VfsSnapshot::file(
            r#"
Key,Source,Context,Example,es
Ack,Ack!,,An exclamation of despair,¡Ay!"#,
        );
        let meta = VfsSnapshot::file(r#"{ "ignoreUnknownInstances": true }"#);

        vfs.debug_load_snapshot("/foo.csv", file);
        vfs.debug_load_snapshot("/foo.meta.json", meta);

        let entry = vfs.get("/foo.csv").unwrap();
        let instance_snapshot =
            SnapshotCsv::from_vfs(&mut InstanceContext::default(), &mut vfs, &entry)
                .unwrap()
                .unwrap();

        assert_yaml_snapshot!(instance_snapshot);
    }
}
