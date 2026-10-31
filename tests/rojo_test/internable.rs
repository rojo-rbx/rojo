use std::collections::HashMap;

use rbx_dom_weak::types::Ref;
use serde::Serialize;

use librojo::web_api::{
    Instance, InstanceUpdate, MessagesPacket, ReadResponse, SocketPacket, SocketPacketBody,
};
use rojo_insta_ext::RedactionMap;

/// A convenience method to store all of the redactable data from a piece of
/// data, then immediately redact it and return a serde_yaml Value.
pub trait InternAndRedact<T> {
    fn intern_and_redact(&self, redactions: &mut RedactionMap, extra: T) -> serde_yaml::Value;
}

impl<I, T> InternAndRedact<T> for I
where
    I: Serialize + Internable<T>,
{
    fn intern_and_redact(&self, redactions: &mut RedactionMap, extra: T) -> serde_yaml::Value {
        self.intern(redactions, extra);
        redactions.redacted_yaml(self)
    }
}

/// A trait to describe how to discover redactable data from an type.
///
/// The 'extra' parameter is a kludge to support types like Instance or
/// ReadResponse that need some additional information in order to be
/// deterministic.
pub trait Internable<T> {
    fn intern(&self, redactions: &mut RedactionMap, extra: T);
}

impl Internable<Ref> for ReadResponse<'_> {
    fn intern(&self, redactions: &mut RedactionMap, root_id: Ref) {
        redactions.intern(root_id);

        let root_instance = self.instances.get(&root_id).unwrap();

        for &child_id in root_instance.children.iter() {
            self.intern(redactions, child_id);
        }
    }
}

impl<'a> Internable<&'a HashMap<Ref, Instance<'_>>> for Instance<'a> {
    fn intern(&self, redactions: &mut RedactionMap, other_instances: &HashMap<Ref, Instance<'_>>) {
        redactions.intern(self.id);

        for child_id in self.children.iter() {
            let child = &other_instances[child_id];
            child.intern(redactions, other_instances);
        }
    }
}

impl Internable<()> for SocketPacket<'_> {
    fn intern(&self, redactions: &mut RedactionMap, extra: ()) {
        redactions.intern(self.session_id);
        match &self.body {
            SocketPacketBody::Messages(packet) => packet.intern(redactions, extra),
        }
    }
}

impl Internable<()> for MessagesPacket<'_> {
    fn intern(&self, redactions: &mut RedactionMap, _extra: ()) {
        for message in &self.messages {
            intern_instance_updates(redactions, &message.updated);
            intern_instance_additions(redactions, &message.added);
        }
    }
}

fn intern_instance_updates(redactions: &mut RedactionMap, updates: &[InstanceUpdate]) {
    for update in updates {
        redactions.intern(update.id);
    }
}

fn intern_instance_additions(
    redactions: &mut RedactionMap,
    additions: &HashMap<Ref, Instance<'_>>,
) {
    // This method redacts in a deterministic order from a HashMap by collecting
    // all of the instances that are direct children of instances we've already
    // interned.
    let mut added_roots = Vec::new();

    for (id, added) in additions {
        let parent_id = added.parent;
        let parent_redacted = redactions.get_redacted_value(parent_id);

        // Here, we assume that instances are only added to other instances that
        // we've already interned. If that's not true, then we'll have some
        // dangling unredacted IDs.
        if let Some(parent_redacted) = parent_redacted {
            added_roots.push((id, parent_redacted));
        }
    }

    // Sort the input by the redacted key, which should match the traversal
    // order we need for the tree.
    added_roots.sort_unstable_by(|a, b| a.1.cmp(&b.1));

    for (root_id, _redacted_id) in added_roots {
        additions[root_id].intern(redactions, additions);
    }
}
