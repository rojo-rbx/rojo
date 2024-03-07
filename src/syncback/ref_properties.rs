//! Implements iterating through an entire WeakDom and linking all Ref
//! properties using attributes.

use std::collections::VecDeque;

use rbx_dom_weak::{
    types::{Attributes, Ref, Variant},
    Instance, WeakDom,
};

use crate::{multimap::MultiMap, REF_ID_ATTRIBUTE_NAME, REF_POINTER_ATTRIBUTE_PREFIX};

pub struct RefRewrites {
    links: MultiMap<Ref, (String, Ref)>,
}

/// Iterates through a WeakDom and collects referent properties.
///
/// They can be linked to a dom later using the `link` method on the returned
/// struct.
pub fn collect_referents(dom: &WeakDom) -> anyhow::Result<RefRewrites> {
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
                    links.insert(referent, (name.clone(), *prop_value))
                }
            }
        }
    }

    Ok(RefRewrites { links })
}

impl RefRewrites {
    /// Links referents for the provided DOM, using the links stored in this
    /// struct.
    pub fn link(self, dom: &mut WeakDom) -> anyhow::Result<()> {
        let mut id_prop_list = Vec::new();

        for (pointer_ref, ref_properties) in self.links {
            for (prop_name, target_ref) in ref_properties {
                let target_inst = dom
                    .get_by_ref_mut(target_ref)
                    .expect("Ref properties that aren't in the DOM should be filtered out");
                let target_attrs = get_or_insert_attributes(target_inst)?;
                if target_attrs.get(REF_ID_ATTRIBUTE_NAME).is_none() {
                    target_attrs.insert(
                        REF_ID_ATTRIBUTE_NAME.to_owned(),
                        Variant::String(target_ref.to_string()),
                    );
                }

                match target_attrs.get(REF_ID_ATTRIBUTE_NAME) {
                    Some(Variant::String(id)) => id_prop_list.push((prop_name, id.clone())),
                    Some(Variant::BinaryString(id)) => match std::str::from_utf8(id.as_ref()) {
                        Ok(str) => id_prop_list.push((prop_name, str.to_string())),
                        Err(_) => anyhow::bail!("expected attribute {REF_ID_ATTRIBUTE_NAME} to be a UTF-8 string")
                    },
                    Some(value) => anyhow::bail!("expected attribute {REF_ID_ATTRIBUTE_NAME} to be a string but it was a {:?}", value.ty()),
                    None => unreachable!("{} should always be inserted as an attribute if it's missing", REF_ID_ATTRIBUTE_NAME),
                }
            }

            let pointer_inst = dom
                .get_by_ref_mut(pointer_ref)
                .expect("invalid referents shouldn't be in the DOM");
            let pointer_attrs = get_or_insert_attributes(pointer_inst)?;
            for (name, id) in id_prop_list.drain(..) {
                pointer_attrs.insert(
                    format!("{REF_POINTER_ATTRIBUTE_PREFIX}{name}"),
                    Variant::BinaryString(id.into_bytes().into()),
                );
            }
        }

        Ok(())
    }
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
