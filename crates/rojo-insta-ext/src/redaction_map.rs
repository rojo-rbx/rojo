use std::collections::HashMap;

use serde::Serialize;

/// Enables redacting any value that serializes as a string.
///
/// Used for transforming Rojo instance IDs into something deterministic.
#[derive(Default)]
pub struct RedactionMap {
    ids: HashMap<String, usize>,
    last_id: usize,
}

impl RedactionMap {
    pub fn get_redacted_value(&self, id: impl ToString) -> Option<String> {
        let id = id.to_string();

        if self.ids.contains_key(&id) {
            Some(id)
        } else {
            None
        }
    }

    /// Returns the numeric ID that was assigned to the provided value,
    /// if one exists.
    pub fn get_id_for_value(&self, value: impl ToString) -> Option<usize> {
        self.ids.get(&value.to_string()).cloned()
    }

    pub fn intern(&mut self, id: impl ToString) {
        let last_id = &mut self.last_id;

        self.ids.entry(id.to_string()).or_insert_with(|| {
            *last_id += 1;
            *last_id
        });
    }

    pub fn intern_iter<S: ToString>(&mut self, ids: impl Iterator<Item = S>) {
        for id in ids {
            self.intern(id.to_string());
        }
    }

    pub fn redacted_yaml(&self, value: impl Serialize) -> serde_yaml::Value {
        let mut encoded = serde_yaml::to_value(value).expect("Couldn't encode value as YAML");

        self.redact(&mut encoded);
        encoded
    }

    pub fn redact(&self, yaml_value: &mut serde_yaml::Value) {
        use serde_yaml::{Mapping, Value};

        match yaml_value {
            Value::String(value) => {
                if let Some(redacted) = self.ids.get(value) {
                    *yaml_value = Value::String(format!("id-{}", *redacted));
                }
            }
            Value::Sequence(sequence) => {
                for value in sequence {
                    self.redact(value);
                }
            }
            Value::Mapping(mapping) => {
                // We can't mutate the keys of a map in-place, so we take
                // ownership of the map and rebuild it.

                let owned_map = std::mem::replace(mapping, Mapping::new());
                let mut new_map = Mapping::with_capacity(owned_map.len());

                for (mut key, mut value) in owned_map {
                    self.redact(&mut key);
                    self.redact(&mut value);
                    new_map.insert(key, value);
                }

                *mapping = new_map;
            }
            _ => {}
        }
    }
}
