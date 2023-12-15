mod hash;
mod variant;

pub use hash::*;
pub use variant::*;

use rbx_dom_weak::{types::Ref, WeakDom};
use std::collections::VecDeque;

pub(crate) fn descendants(dom: &WeakDom) -> Vec<Ref> {
    let mut queue = VecDeque::new();
    let mut ordered = Vec::new();
    queue.push_front(dom.root_ref());

    while let Some(referent) = queue.pop_front() {
        let inst = dom
            .get_by_ref(referent)
            .expect("Invariant: WeakDom had a Ref that wasn't inside it");
        ordered.push(referent);
        for child in inst.children() {
            queue.push_back(*child)
        }
    }

    ordered
}
