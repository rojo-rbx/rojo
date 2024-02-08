//! Implements iterating through an entire WeakDom and linking all Ref
//! properties using attributes.

use std::collections::VecDeque;

use rbx_dom_weak::{
    types::{Attributes, Variant},
    Instance, WeakDom,
};

use crate::{multimap::MultiMap, REF_ID_ATTRIBUTE_NAME, REF_POINTER_ATTRIBUTE_PREFIX};

use super::get_inst_path;

/// Iterates through a WeakDom and links referent properties using attributes.
pub fn link_referents(dom: &mut WeakDom) -> anyhow::Result<()> {
    let mut links = MultiMap::new();
    let mut queue = VecDeque::new();

    // Note that this is back-in, front-out. This is important because
    // VecDeque::extend is the equivalent to using push_back.
    queue.push_back(dom.root_ref());
    while let Some(referent) = queue.pop_front() {
        let instance = dom.get_by_ref(referent).unwrap();

        queue.extend(instance.children().iter().copied());
        for (name, value) in &instance.properties {
            if let Variant::Ref(prop_value) = value {
                if dom.get_by_ref(*prop_value).is_some() {
                    log::trace!(
                        "{}.{name} -> {}",
                        get_inst_path(dom, referent),
                        get_inst_path(dom, *prop_value)
                    );
                    links.insert(referent, (name.clone(), *prop_value))
                }
            }
        }
    }
    let mut rewrites = Vec::new();

    for (pointer_ref, ref_properties) in links {
        for (prop_name, target_ref) in ref_properties {
            log::debug!(
                "Linking {}.{prop_name} to {} ({pointer_ref} to {target_ref})",
                dom.get_by_ref(pointer_ref).unwrap().name,
                dom.get_by_ref(target_ref).unwrap().name,
            );
            let target_inst = dom
                .get_by_ref_mut(target_ref)
                .expect("Ref properties that aren't in DOM should be filtered");

            let attributes = get_or_insert_attributes(target_inst)?;
            if attributes.get(REF_ID_ATTRIBUTE_NAME).is_none() {
                attributes.insert(
                    REF_ID_ATTRIBUTE_NAME.to_owned(),
                    Variant::String(target_ref.to_string()),
                );
            }

            let target_id = attributes
                .get(REF_ID_ATTRIBUTE_NAME)
                .expect("every Instance to have an ID");
            if let Variant::String(value) = target_id {
                rewrites.push((prop_name, value.clone().into_bytes()));
            } else if let Variant::BinaryString(value) = target_id {
                rewrites.push((prop_name, value.clone().into_vec()))
            }
        }

        let inst = dom.get_by_ref_mut(pointer_ref).unwrap();
        let attrs = get_or_insert_attributes(inst)?;
        for (name, id) in rewrites.drain(..) {
            attrs.insert(
                format!("{REF_POINTER_ATTRIBUTE_PREFIX}{name}"),
                String::from_utf8(id).unwrap().into(),
            );
        }
    }

    Ok(())
}

fn get_or_insert_attributes(inst: &mut Instance) -> anyhow::Result<&mut Attributes> {
    if !inst.properties.contains_key("Attributes") {
        inst.properties
            .insert("Attributes".into(), Attributes::new().into());
    }
    match inst.properties.get_mut("Attributes") {
        Some(Variant::Attributes(attrs)) => Ok(attrs),
        Some(ty) => Err(anyhow::format_err!(
            "expected property Attributes to be an Attributes but it was {:?}",
            ty.ty()
        )),
        None => unreachable!(),
    }
}
