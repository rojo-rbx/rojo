use std::{collections::HashMap, path::PathBuf};

#[derive(Debug)]
pub struct GenerationMap {
    known: HashMap<PathBuf, bool>,
    updatable: PathBuf,
    new: HashMap<PathBuf, bool>,
    bypass: bool,
}
impl GenerationMap {
    pub fn new() -> GenerationMap {
        Self {
            known: HashMap::new(),
            updatable: PathBuf::new(),
            new: HashMap::new(),

            //Is for project files to bypass
            bypass: false,
        }
    }

    pub fn next_generation(&mut self, path: PathBuf) {
        self.bypass = false;
        for (k, _) in self.new.iter() {
            self.known.insert(k.clone(), true);
        }
        self.new = HashMap::new();
        self.updatable = path
    }
    pub fn bypass(&mut self) {
        self.bypass = true
    }
    pub fn should_ignore_chilren(&mut self, path: PathBuf) -> bool {
        let option = self.known.get(&path);
        match option {
            Some(_) => {
                if self.updatable != path {
                    if self.bypass {
                        return false;
                    }
                    return true;
                }
            }
            None => {
                self.new.insert(path, true);
            }
        }
        false
    }
}
