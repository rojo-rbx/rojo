use std::borrow::Cow;

use maplit::hashmap;
use rbx_dom_weak::RbxValue;

use crate::{
    imfs::{Imfs, ImfsFile},
    snapshot::InstanceSnapshot,
};

use super::{
    error::{SnapshotResult, SnapshotError},
};

pub fn snapshot<'source>(
    file: &'source ImfsFile,
    _imfs: &'source Imfs,
) -> SnapshotResult<'source> {
    let instance_name = file.path
        .file_stem().expect("Could not extract file stem")
        .to_str().ok_or_else(|| SnapshotError::file_name_bad_unicode(&file.path))?;

    let contents = std::str::from_utf8(&file.contents)
        .map_err(|inner| SnapshotError::file_contents_bad_unicode(inner, &file.path))?;

    let properties = hashmap! {
        "Value".to_owned() => RbxValue::String {
            value: contents.to_owned(),
        },
    };

    let mut snapshot = InstanceSnapshot {
        snapshot_id: None,
        name: Cow::Borrowed(instance_name),
        class_name: Cow::Borrowed("StringValue"),
        properties,
        children: Vec::new(),
        // metadata: MetadataPerInstance {
        //     source_path: Some(file.path.to_path_buf()),
        //     ignore_unknown_instances: false,
        //     project_definition: None,
        // },
    };

    // if let Some(meta) = ExtraMetadata::locate(&imfs, &file.path)? {
    //     meta.validate_for_non_init(&file.path)?;
    //     meta.apply(&mut snapshot)?;
    // }

    Ok(Some(snapshot))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn smoke_test() {
        // TODO
    }
}