//! Defines the algorithm for computing a roughly-minimal patch set given an
//! existing instance tree and an instance snapshot.

use std::{
    collections::{HashMap, HashSet},
    mem::take,
};

use rbx_dom_weak::types::{
    CFrame, Matrix3, NumberSequenceKeypoint, PhysicalProperties, Ref, UDim, Variant, Vector2,
    Vector3,
};

use super::{
    patch::{PatchAdd, PatchSet, PatchUpdate},
    InstanceSnapshot, InstanceWithMeta, RojoTree,
};

#[profiling::function]
pub fn compute_patch_set(snapshot: Option<InstanceSnapshot>, tree: &RojoTree, id: Ref) -> PatchSet {
    let mut patch_set = PatchSet::new();

    if let Some(snapshot) = snapshot {
        let mut context = ComputePatchContext::default();

        compute_patch_set_internal(&mut context, snapshot, tree, id, &mut patch_set);

        // Rewrite Ref properties to refer to instance IDs instead of snapshot IDs
        // for all of the IDs that we know about so far.
        rewrite_refs_in_updates(&context, &mut patch_set.updated_instances);
        rewrite_refs_in_additions(&context, &mut patch_set.added_instances);
    } else if id != tree.get_root_id() {
        patch_set.removed_instances.push(id);
    }

    patch_set
}

#[derive(Default)]
struct ComputePatchContext {
    snapshot_id_to_instance_id: HashMap<Ref, Ref>,
}

fn rewrite_refs_in_updates(context: &ComputePatchContext, updates: &mut [PatchUpdate]) {
    for update in updates {
        for property_value in update.changed_properties.values_mut() {
            if let Some(Variant::Ref(referent)) = property_value {
                if let Some(&instance_ref) = context.snapshot_id_to_instance_id.get(referent) {
                    *property_value = Some(Variant::Ref(instance_ref));
                }
            }
        }
    }
}

fn rewrite_refs_in_additions(context: &ComputePatchContext, additions: &mut [PatchAdd]) {
    for addition in additions {
        rewrite_refs_in_snapshot(context, &mut addition.instance);
    }
}

fn rewrite_refs_in_snapshot(context: &ComputePatchContext, snapshot: &mut InstanceSnapshot) {
    for property_value in snapshot.properties.values_mut() {
        if let Variant::Ref(referent) = property_value {
            if let Some(&instance_referent) = context.snapshot_id_to_instance_id.get(referent) {
                *property_value = Variant::Ref(instance_referent);
            }
        }
    }

    for child in &mut snapshot.children {
        rewrite_refs_in_snapshot(context, child);
    }
}

fn compute_patch_set_internal(
    context: &mut ComputePatchContext,
    mut snapshot: InstanceSnapshot,
    tree: &RojoTree,
    id: Ref,
    patch_set: &mut PatchSet,
) {
    if snapshot.snapshot_id.is_some() {
        context
            .snapshot_id_to_instance_id
            .insert(snapshot.snapshot_id, id);
    }

    let instance = tree
        .get_instance(id)
        .expect("Instance did not exist in tree");

    compute_property_patches(&mut snapshot, &instance, patch_set);
    compute_children_patches(context, &mut snapshot, tree, id, patch_set);
}

fn compute_property_patches(
    snapshot: &mut InstanceSnapshot,
    instance: &InstanceWithMeta,
    patch_set: &mut PatchSet,
) {
    let mut visited_properties = HashSet::new();
    let mut changed_properties = HashMap::new();

    let changed_name = if snapshot.name == instance.name() {
        None
    } else {
        Some(take(&mut snapshot.name).into_owned())
    };

    let changed_class_name = if snapshot.class_name == instance.class_name() {
        None
    } else {
        Some(take(&mut snapshot.class_name).into_owned())
    };

    let changed_metadata = if &snapshot.metadata == instance.metadata() {
        None
    } else {
        Some(take(&mut snapshot.metadata))
    };

    for (name, snapshot_value) in take(&mut snapshot.properties) {
        visited_properties.insert(name.clone());

        match instance.properties().get(&name) {
            Some(instance_value) => {
                if snapshot_value.different(instance_value) {
                    changed_properties.insert(name, Some(snapshot_value));
                }
            }
            None => {
                changed_properties.insert(name, Some(snapshot_value));
            }
        }
    }

    for name in instance.properties().keys() {
        if visited_properties.contains(name.as_str()) {
            continue;
        }

        changed_properties.insert(name.clone(), None);
    }

    if changed_properties.is_empty()
        && changed_name.is_none()
        && changed_class_name.is_none()
        && changed_metadata.is_none()
    {
        return;
    }

    patch_set.updated_instances.push(PatchUpdate {
        id: instance.id(),
        changed_name,
        changed_class_name,
        changed_properties,
        changed_metadata,
    });
}

fn compute_children_patches(
    context: &mut ComputePatchContext,
    snapshot: &mut InstanceSnapshot,
    tree: &RojoTree,
    id: Ref,
    patch_set: &mut PatchSet,
) {
    let instance = tree
        .get_instance(id)
        .expect("Instance did not exist in tree");

    let instance_children = instance.children();

    let mut paired_instances = vec![false; instance_children.len()];

    for snapshot_child in take(&mut snapshot.children) {
        let matching_instance =
            instance_children
                .iter()
                .enumerate()
                .find(|(instance_index, instance_child_id)| {
                    if paired_instances[*instance_index] {
                        return false;
                    }

                    let instance_child = tree
                        .get_instance(**instance_child_id)
                        .expect("Instance did not exist in tree");

                    if snapshot_child.name == instance_child.name()
                        && snapshot_child.class_name == instance_child.class_name()
                    {
                        paired_instances[*instance_index] = true;
                        return true;
                    }

                    false
                });

        match matching_instance {
            Some((_, instance_child_id)) => {
                compute_patch_set_internal(
                    context,
                    snapshot_child,
                    tree,
                    *instance_child_id,
                    patch_set,
                );
            }
            None => {
                patch_set.added_instances.push(PatchAdd {
                    parent_id: id,
                    instance: snapshot_child,
                });
            }
        }
    }

    for (instance_index, instance_child_id) in instance_children.iter().enumerate() {
        if paired_instances[instance_index] {
            continue;
        }

        patch_set.removed_instances.push(*instance_child_id);
    }
}

/// Trait where NaN values must not be treated as different.
trait Different {
    fn different(&self, b: &Self) -> bool;
}

impl Different for Variant {
    fn different(&self, b: &Self) -> bool {
        match (self, b) {
            (Variant::CFrame(a), Variant::CFrame(b)) => a.different(b),
            (Variant::Float32(a), Variant::Float32(b)) => a.different(b),
            (Variant::Float64(a), Variant::Float64(b)) => a.different(b),
            (Variant::NumberRange(a), Variant::NumberRange(b)) => {
                a.min.different(&b.min) || a.max.different(&b.max)
            }
            (Variant::NumberSequence(a), Variant::NumberSequence(b)) => {
                if a.keypoints.len() != b.keypoints.len() {
                    return true;
                }

                for i in 0..a.keypoints.len() {
                    if a.keypoints[i].different(&b.keypoints[i]) {
                        return true;
                    }
                }

                false
            }
            (
                Variant::PhysicalProperties(PhysicalProperties::Custom(a)),
                Variant::PhysicalProperties(PhysicalProperties::Custom(b)),
            ) => {
                a.density.different(&b.density)
                    || a.elasticity.different(&b.elasticity)
                    || a.elasticity_weight.different(&b.elasticity_weight)
                    || a.friction.different(&b.friction)
                    || a.friction_weight.different(&b.friction_weight)
            }
            (Variant::Ray(a), Variant::Ray(b)) => {
                a.direction.different(&b.direction) || a.origin.different(&b.origin)
            }
            (Variant::Rect(a), Variant::Rect(b)) => {
                a.min.different(&b.min) || a.max.different(&b.max)
            }
            (Variant::Region3(a), Variant::Region3(b)) => {
                a.min.different(&b.min) || a.max.different(&b.max)
            }
            (Variant::UDim(a), Variant::UDim(b)) => a.different(b),
            (Variant::UDim2(a), Variant::UDim2(b)) => a.x.different(&b.x) || a.y.different(&b.y),
            (Variant::Vector2(a), Variant::Vector2(b)) => a.different(b),
            (Variant::Vector3(a), Variant::Vector3(b)) => a.different(b),
            (Variant::OptionalCFrame(Some(a)), Variant::OptionalCFrame(Some(b))) => a.different(b),
            (Variant::Attributes(a), Variant::Attributes(b)) => {
                a.len() != b.len()
                    || a.iter()
                        .zip(b.iter())
                        .any(|((a_name, a_value), (b_name, b_value))| {
                            a_name != b_name || a_value.different(b_value)
                        })
            }
            _ => self != b,
        }
    }
}

impl Different for f32 {
    fn different(&self, b: &Self) -> bool {
        if self.is_nan() && b.is_nan() {
            return false;
        }

        self != b
    }
}

impl Different for f64 {
    fn different(&self, b: &Self) -> bool {
        if self.is_nan() && b.is_nan() {
            return false;
        }

        self != b
    }
}

impl Different for UDim {
    fn different(&self, b: &Self) -> bool {
        self.offset != b.offset || self.scale.different(&b.scale)
    }
}

impl Different for Vector2 {
    fn different(&self, b: &Self) -> bool {
        self.x.different(&b.x) || self.y.different(&b.y)
    }
}

impl Different for Vector3 {
    fn different(&self, b: &Self) -> bool {
        self.x.different(&b.x) || self.y.different(&b.y) || self.z.different(&b.z)
    }
}

impl Different for CFrame {
    fn different(&self, b: &Self) -> bool {
        self.position.different(&b.position) || self.orientation.different(&b.orientation)
    }
}

impl Different for Matrix3 {
    fn different(&self, b: &Self) -> bool {
        self.x.different(&b.x) || self.y.different(&b.y) || self.z.different(&b.z)
    }
}

impl Different for NumberSequenceKeypoint {
    fn different(&self, b: &Self) -> bool {
        self.envelope.different(&b.envelope)
            || self.time.different(&b.time)
            || self.value.different(&b.time)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use std::borrow::Cow;

    use maplit::hashmap;

    /// This test makes sure that rewriting refs in instance update patches to
    /// instances that already exists works. We should be able to correlate the
    /// snapshot ID and instance ID during patch computation and replace the
    /// value before returning from compute_patch_set.
    #[test]
    fn rewrite_ref_existing_instance_update() {
        let tree = RojoTree::new(InstanceSnapshot::new().name("foo").class_name("foo"));

        let root_id = tree.get_root_id();

        // This snapshot should be identical to the existing tree except for the
        // addition of a prop named Self, which is a self-referential Ref.
        let snapshot_id = Ref::new();
        let snapshot = InstanceSnapshot {
            snapshot_id,
            properties: hashmap! {
                "Self".to_owned() => Variant::Ref(snapshot_id),
            },

            metadata: Default::default(),
            name: Cow::Borrowed("foo"),
            class_name: Cow::Borrowed("foo"),
            children: Vec::new(),
        };

        let patch_set = compute_patch_set(Some(snapshot), &tree, root_id);

        let expected_patch_set = PatchSet {
            updated_instances: vec![PatchUpdate {
                id: root_id,
                changed_name: None,
                changed_class_name: None,
                changed_properties: hashmap! {
                    "Self".to_owned() => Some(Variant::Ref(root_id)),
                },
                changed_metadata: None,
            }],
            added_instances: Vec::new(),
            removed_instances: Vec::new(),
        };

        assert_eq!(patch_set, expected_patch_set);
    }

    /// The same as rewrite_ref_existing_instance_update, except that the
    /// property is added in a new instance instead of modifying an existing
    /// one.
    #[test]
    fn rewrite_ref_existing_instance_addition() {
        let tree = RojoTree::new(InstanceSnapshot::new().name("foo").class_name("foo"));

        let root_id = tree.get_root_id();

        // This patch describes the existing instance with a new child added.
        let snapshot_id = Ref::new();
        let snapshot = InstanceSnapshot {
            snapshot_id,
            children: vec![InstanceSnapshot {
                properties: hashmap! {
                    "Self".to_owned() => Variant::Ref(snapshot_id),
                },

                snapshot_id: Ref::none(),
                metadata: Default::default(),
                name: Cow::Borrowed("child"),
                class_name: Cow::Borrowed("child"),
                children: Vec::new(),
            }],

            metadata: Default::default(),
            properties: HashMap::new(),
            name: Cow::Borrowed("foo"),
            class_name: Cow::Borrowed("foo"),
        };

        let patch_set = compute_patch_set(Some(snapshot), &tree, root_id);

        let expected_patch_set = PatchSet {
            added_instances: vec![PatchAdd {
                parent_id: root_id,
                instance: InstanceSnapshot {
                    snapshot_id: Ref::none(),
                    metadata: Default::default(),
                    properties: hashmap! {
                        "Self".to_owned() => Variant::Ref(root_id),
                    },
                    name: Cow::Borrowed("child"),
                    class_name: Cow::Borrowed("child"),
                    children: Vec::new(),
                },
            }],
            updated_instances: Vec::new(),
            removed_instances: Vec::new(),
        };

        assert_eq!(patch_set, expected_patch_set);
    }

    #[test]
    fn different() {
        assert!(5.0.different(&6.0));
        assert!(!5.0.different(&5.0));
        assert!(!f32::NAN.different(&f32::NAN));
        assert!(f32::NAN.different(&5.0));
    }
}
