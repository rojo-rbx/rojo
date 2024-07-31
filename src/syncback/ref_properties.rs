//! Implements iterating through an entire WeakDom and linking all Ref
//! properties using attributes.

use std::collections::{hash_map::Entry, HashMap, VecDeque};

use rbx_dom_weak::{
    types::{Attributes, Ref, UniqueId, Variant},
    Instance, WeakDom,
};

use crate::{multimap::MultiMap, REF_ID_ATTRIBUTE_NAME, REF_POINTER_ATTRIBUTE_PREFIX};

pub struct RefLinks {
    /// A map of referents to each of their Ref properties.
    prop_links: MultiMap<Ref, RefLink>,
    /// A set of referents that need their ID rewritten. This includes
    /// Instances that have no existing ID.
    need_rewrite: Vec<Ref>,
}

#[derive(PartialEq, Eq)]
struct RefLink {
    /// The name of a property
    name: String,
    /// The value of the property.
    value: Ref,
}

/// Iterates through a WeakDom and collects referent properties.
///
/// They can be linked to a dom later using `link_referents`.
pub fn collect_referents(dom: &WeakDom) -> RefLinks {
    let mut existing_ids = HashMap::new();
    let mut need_rewrite = Vec::new();
    let mut links = MultiMap::new();

    let mut queue = VecDeque::new();

    // Note that this is back-in, front-out. This is important because
    // VecDeque::extend is the equivalent to using push_back.
    queue.push_back(dom.root_ref());
    while let Some(inst_ref) = queue.pop_front() {
        let instance = dom.get_by_ref(inst_ref).unwrap();
        queue.extend(instance.children().iter().copied());

        // Collect all referent properties for easy access later
        for (property_name, prop_value) in &instance.properties {
            if let Variant::Ref(prop_ref) = prop_value {
                // Any Instance that's pointed to as a property needs an ID.
                let existing_id = match dom.get_by_ref(*prop_ref) {
                    Some(inst) => get_existing_id(inst),
                    None => continue,
                };
                if let Some(existing_id) = existing_id {
                    match existing_ids.entry(existing_id) {
                        Entry::Occupied(entry) => {
                            if entry.get() != prop_ref {
                                need_rewrite.push(*prop_ref);
                            }
                        }
                        Entry::Vacant(entry) => {
                            entry.insert(*prop_ref);
                        }
                    }
                } else {
                    need_rewrite.push(*prop_ref);
                }
                // We also need a list of these properties for linking later
                links.insert(
                    inst_ref,
                    RefLink {
                        name: property_name.to_owned(),
                        value: *prop_ref,
                    },
                );
            }
        }
    }

    RefLinks {
        prop_links: links,
        need_rewrite,
    }
}

pub fn link_referents(links: RefLinks, dom: &mut WeakDom) -> anyhow::Result<()> {
    write_id_attributes(&links, dom)?;

    let mut prop_list = Vec::new();
    let mut attribute_list = HashMap::new();

    for (inst_id, properties) in links.prop_links {
        for ref_link in properties {
            let prop_inst = match dom.get_by_ref(ref_link.value) {
                Some(inst) => inst,
                None => continue,
            };
            let id = get_existing_id(prop_inst)
                .expect("all Instances that are pointed to should have an ID");
            prop_list.push((ref_link.name, Variant::String(id.to_owned())));
        }
        let inst = match dom.get_by_ref_mut(inst_id) {
            Some(inst) => inst,
            None => continue,
        };

        // TODO: Replace this whole rigamarole with `Attributes::drain`
        // eventually.
        let attributes = match inst.properties.remove("Attributes") {
            Some(Variant::Attributes(attrs)) => attrs,
            None => Attributes::new(),
            Some(value) => {
                anyhow::bail!(
                    "expected Attributes to be of type 'Attributes' but it was of type '{:?}'",
                    value.ty()
                );
            }
        };
        for (name, value) in attributes.into_iter() {
            if !name.starts_with(REF_POINTER_ATTRIBUTE_PREFIX) {
                attribute_list.insert(name, value);
            }
        }

        for (prop_name, prop_value) in prop_list.drain(..) {
            attribute_list.insert(
                format!("{REF_POINTER_ATTRIBUTE_PREFIX}{prop_name}"),
                prop_value,
            );
        }

        // TODO: Same as above, when `Attributes::drain` is live, replace this
        // with it.
        inst.properties.insert(
            "Attributes".into(),
            Attributes::from_iter(attribute_list.drain()).into(),
        );
    }

    Ok(())
}

fn write_id_attributes(links: &RefLinks, dom: &mut WeakDom) -> anyhow::Result<()> {
    for referent in &links.need_rewrite {
        let inst = match dom.get_by_ref_mut(*referent) {
            Some(inst) => inst,
            None => continue,
        };
        let unique_id = match inst.properties.get("UniqueId") {
            Some(Variant::UniqueId(id)) => Some(*id),
            _ => None,
        }
        .unwrap_or_else(|| UniqueId::now().unwrap());

        let attributes = match inst.properties.get_mut("Attributes") {
            Some(Variant::Attributes(attrs)) => attrs,
            None => {
                inst.properties
                    .insert("Attributes".into(), Attributes::new().into());
                match inst.properties.get_mut("Attributes") {
                    Some(Variant::Attributes(attrs)) => attrs,
                    _ => unreachable!(),
                }
            }
            Some(value) => {
                anyhow::bail!(
                    "expected Attributes to be of type 'Attributes' but it was of type '{:?}'",
                    value.ty()
                );
            }
        };
        attributes.insert(
            REF_ID_ATTRIBUTE_NAME.into(),
            Variant::String(unique_id.to_string()),
        );
    }
    Ok(())
}

fn get_existing_id(inst: &Instance) -> Option<&str> {
    if let Variant::Attributes(attrs) = inst.properties.get("Attributes")? {
        let id = attrs.get(REF_ID_ATTRIBUTE_NAME)?;
        match id {
            Variant::String(str) => Some(str),
            Variant::BinaryString(bstr) => match std::str::from_utf8(bstr.as_ref()) {
                Ok(str) => Some(str),
                Err(_) => None,
            },
            _ => None,
        }
    } else {
        None
    }
}