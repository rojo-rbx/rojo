//! Implements iterating through an entire WeakDom and linking all Ref
//! properties using attributes.

use std::collections::{HashMap, VecDeque};

use rbx_dom_weak::{
    types::{Attributes, Ref, Variant},
    Instance, WeakDom,
};

use crate::{multimap::MultiMap, REF_ID_ATTRIBUTE_NAME, REF_POINTER_ATTRIBUTE_PREFIX};

#[derive(PartialEq, Eq)]
pub struct RefLink {
    /// The name of a property
    name: String,
    /// The value of the property.
    value: Ref,
}

/// Iterates through a WeakDom and collects referent properties.
///
/// They can be linked to a dom later using the `link` method on the returned
/// struct.
pub fn collect_referents(dom: &WeakDom) -> anyhow::Result<MultiMap<Ref, RefLink>> {
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
                    links.insert(
                        referent,
                        RefLink {
                            name: name.clone(),
                            value: *prop_value,
                        },
                    );
                }
            }
        }
    }

    Ok(links)
}

pub fn link_referents(link_list: MultiMap<Ref, RefLink>, dom: &mut WeakDom) -> anyhow::Result<()> {
    let mut pointer_attributes = HashMap::new();
    for (pointer_ref, ref_properties) in link_list {
        if dom.get_by_ref(pointer_ref).is_none() {
            continue;
        }

        // In this loop, we need to add the `Rojo_Id` attributes to the
        // Instances.
        for ref_link in ref_properties {
            let target_inst = match dom.get_by_ref_mut(ref_link.value) {
                Some(inst) => inst,
                None => {
                    continue;
                }
            };
            pointer_attributes.insert(ref_link.name, get_or_insert_id(target_inst)?);
        }

        let pointer_inst = dom.get_by_ref_mut(pointer_ref).unwrap();
        let pointer_attrs = get_or_insert_attributes(pointer_inst)?;
        for (name, id) in pointer_attributes.drain() {
            pointer_attrs.insert(
                format!("{REF_POINTER_ATTRIBUTE_PREFIX}{name}"),
                Variant::BinaryString(id.into_bytes().into()),
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

fn get_or_insert_id(inst: &mut Instance) -> anyhow::Result<String> {
    let unique_id = match inst.properties.get("UniqueId") {
        Some(Variant::UniqueId(id)) => Some(*id),
        _ => None,
    };
    let referent = inst.referent();
    let attributes = get_or_insert_attributes(inst)?;
    match attributes.get(REF_ID_ATTRIBUTE_NAME) {
        Some(Variant::String(str)) => return Ok(str.clone()),
        Some(Variant::BinaryString(bytes)) => match std::str::from_utf8(bytes.as_ref()) {
            Ok(str) => return Ok(str.to_string()),
            Err(_) => {
                anyhow::bail!("expected attribute {REF_ID_ATTRIBUTE_NAME} to be a UTF-8 string")
            }
        },
        _ => {}
    }
    let id_string = unique_id
        .map(|id| id.to_string())
        .unwrap_or_else(|| referent.to_string());
    attributes.insert(
        REF_ID_ATTRIBUTE_NAME.to_string(),
        Variant::BinaryString(id_string.clone().into_bytes().into()),
    );
    Ok(id_string)
}
