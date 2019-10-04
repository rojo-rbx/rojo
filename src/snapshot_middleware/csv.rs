use std::{borrow::Cow, collections::BTreeMap};

use maplit::hashmap;
use rbx_dom_weak::{RbxId, RbxTree, RbxValue};
use serde::Serialize;

use crate::{
    imfs::{Imfs, ImfsEntry, ImfsFetcher},
    snapshot::{InstanceMetadata, InstanceSnapshot},
};

use super::middleware::{SnapshotFileResult, SnapshotInstanceResult, SnapshotMiddleware};

pub struct SnapshotCsv;

impl SnapshotMiddleware for SnapshotCsv {
    fn from_imfs<F: ImfsFetcher>(
        imfs: &mut Imfs<F>,
        entry: &ImfsEntry,
    ) -> SnapshotInstanceResult<'static> {
        if entry.is_directory() {
            return Ok(None);
        }

        let file_name = entry.path().file_name().unwrap().to_string_lossy();

        if !file_name.ends_with(".csv") {
            return Ok(None);
        }

        let instance_name = entry
            .path()
            .file_stem()
            .expect("Could not extract file stem")
            .to_string_lossy()
            .to_string();

        let table_contents = convert_localization_csv(entry.contents(imfs)?);

        Ok(Some(InstanceSnapshot {
            snapshot_id: None,
            metadata: InstanceMetadata {
                contributing_paths: vec![entry.path().to_path_buf()],
                ..Default::default()
            },
            name: Cow::Owned(instance_name),
            class_name: Cow::Borrowed("LocalizationTable"),
            properties: hashmap! {
                "Contents".to_owned() => RbxValue::String {
                    value: table_contents,
                },
            },
            children: Vec::new(),
        }))
    }

    fn from_instance(_tree: &RbxTree, _id: RbxId) -> SnapshotFileResult {
        unimplemented!("Snapshotting CSV localization tables");
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

    use crate::imfs::{ImfsDebug, ImfsSnapshot, NoopFetcher};
    use insta::assert_yaml_snapshot;

    #[test]
    fn csv_from_imfs() {
        let mut imfs = Imfs::new(NoopFetcher);
        let file = ImfsSnapshot::file(
            r#"
Key,Source,Context,Example,es
Ack,Ack!,,An exclamation of despair,¡Ay!"#,
        );

        imfs.debug_load_snapshot("/foo.csv", file);

        let entry = imfs.get("/foo.csv").unwrap();
        let instance_snapshot = SnapshotCsv::from_imfs(&mut imfs, &entry).unwrap().unwrap();

        assert_yaml_snapshot!(instance_snapshot);
    }
}
