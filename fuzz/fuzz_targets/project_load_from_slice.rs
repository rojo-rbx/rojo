#![no_main]

use std::path::Path;

use libfuzzer_sys::fuzz_target;
use librojo::Project;

fuzz_target!(|data: &[u8]| {
    let _ = Project::load_from_slice(data, Path::new("test.project.json"));
});