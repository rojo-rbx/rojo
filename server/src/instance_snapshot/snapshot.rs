use std::{
    borrow::Cow,
    collections::HashMap,
};

use rbx_dom_weak::RbxValue;

#[derive(Debug, Clone)]
pub struct InstanceSnapshot<'a> {
    pub name: Cow<'a, str>,
    pub class_name: Cow<'a, str>,
    pub properties: HashMap<String, RbxValue>,
    pub children: Vec<InstanceSnapshot<'a>>,
}

impl<'a> InstanceSnapshot<'a> {
    pub fn get_owned(&'a self) -> InstanceSnapshot<'static> {
        let children: Vec<InstanceSnapshot<'static>> = self.children.iter()
            .map(InstanceSnapshot::get_owned)
            .collect();

        InstanceSnapshot {
            name: Cow::Owned(self.name.clone().into_owned()),
            class_name: Cow::Owned(self.class_name.clone().into_owned()),
            properties: self.properties.clone(),
            children,
        }
    }
}