//! Implements iterating through an entire WeakDom and linking all Ref
//! properties using attributes.

use std::collections::{HashMap, HashSet, VecDeque};

use rbx_dom_weak::{
    types::{Attributes, Ref, UniqueId, Variant},
    ustr, Instance, WeakDom,
};

use crate::{multimap::MultiMap, REF_ID_ATTRIBUTE_NAME, REF_POINTER_ATTRIBUTE_PREFIX};

pub struct RefLinks {
    /// A map of referents to each of their Ref properties.
    prop_links: MultiMap<Ref, RefLink>,
    /// A set of referents that need their ID rewritten. This includes
    /// Instances that have no existing ID.
    need_rewrite: HashSet<Ref>,
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
    let mut ids = HashMap::new();
    let mut need_rewrite = HashSet::new();
    let mut links = MultiMap::new();

    // Note that this is back-in, front-out. This is important because
    // VecDeque::extend is the equivalent to using push_back.
    let mut queue = VecDeque::new();
    queue.push_back(dom.root_ref());
    while let Some(inst_ref) = queue.pop_front() {
        let pointer = dom.get_by_ref(inst_ref).unwrap();
        queue.extend(pointer.children().iter().copied());

        for (prop_name, prop_value) in &pointer.properties {
            let Variant::Ref(prop_value) = prop_value else {
                continue;
            };

            links.insert(
                inst_ref,
                RefLink {
                    name: prop_name.to_string(),
                    value: *prop_value,
                },
            );

            let target = dom
                .get_by_ref(*prop_value)
                .expect("Refs in DOM should point to valid Instances");

            // 1. Check if target has an ID
            if let Some(id) = get_existing_id(target) {
                // If it does, we need to check whether that ID is a duplicate
                if let Some(id_ref) = ids.get(id) {
                    // If the same ID points to a new Instance, rewrite it.
                    if id_ref != prop_value {
                        if log::log_enabled!(log::Level::Trace) {
                            log::trace!(
                                "{} needs an id rewritten because it has the same id as {}",
                                target.name,
                                dom.get_by_ref(*id_ref).unwrap().name
                            );
                        }
                        need_rewrite.insert(*prop_value);
                    }
                }
                ids.insert(id, *prop_value);
            } else {
                log::trace!("{} needs an id rewritten because it has no id but is referred to by {}.{prop_name}", target.name, pointer.name);
                // If it does not, it needs one.
                need_rewrite.insert(*prop_value);
            }
        }
    }

    RefLinks {
        need_rewrite,
        prop_links: links,
    }
}

pub fn link_referents(links: RefLinks, dom: &mut WeakDom) -> anyhow::Result<()> {
    write_id_attributes(&links, dom)?;

    let mut prop_list = Vec::new();

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

        let mut attributes: Attributes = match inst.properties.remove(&ustr("Attributes")) {
            Some(Variant::Attributes(attrs)) => attrs,
            None => Attributes::new(),
            Some(value) => {
                anyhow::bail!(
                    "expected Attributes to be of type 'Attributes' but it was of type '{:?}'",
                    value.ty()
                );
            }
        }
        .into_iter()
        .filter(|(name, _)| !name.starts_with(REF_POINTER_ATTRIBUTE_PREFIX))
        .collect();

        for (prop_name, prop_value) in prop_list.drain(..) {
            attributes.insert(
                format!("{REF_POINTER_ATTRIBUTE_PREFIX}{prop_name}"),
                prop_value,
            );
        }

        inst.properties
            .insert("Attributes".into(), attributes.into());
    }

    Ok(())
}

fn write_id_attributes(links: &RefLinks, dom: &mut WeakDom) -> anyhow::Result<()> {
    for referent in &links.need_rewrite {
        let inst = match dom.get_by_ref_mut(*referent) {
            Some(inst) => inst,
            None => continue,
        };
        let unique_id = match inst.properties.get(&ustr("UniqueId")) {
            Some(Variant::UniqueId(id)) => Some(*id),
            _ => None,
        }
        .unwrap_or_else(|| UniqueId::now().unwrap());

        let attributes = match inst.properties.get_mut(&ustr("Attributes")) {
            Some(Variant::Attributes(attrs)) => attrs,
            None => {
                inst.properties
                    .insert("Attributes".into(), Attributes::new().into());
                match inst.properties.get_mut(&ustr("Attributes")) {
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
    if let Variant::Attributes(attrs) = inst.properties.get(&ustr("Attributes"))? {
        let id = attrs.get(REF_ID_ATTRIBUTE_NAME)?;
        match id {
            Variant::String(str) => Some(str),
            Variant::BinaryString(bstr) => std::str::from_utf8(bstr.as_ref()).ok(),
            _ => None,
        }
    } else {
        None
    }
}
