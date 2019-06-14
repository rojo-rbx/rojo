use std::collections::{HashMap, HashSet};

use rbx_dom_weak::{RbxTree, RbxId, RbxInstance};

use super::{
    snapshot::InstanceSnapshot,
    patch::{PatchSet, PatchChildren, PatchChildrenEntry, PatchProperties},
};

pub fn apply_patch(
    tree: &RbxTree,
    id: RbxId,
    patch_set: &PatchSet,
) {

}