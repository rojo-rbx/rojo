use std::{ffi::CStr, os::raw::c_char, path::PathBuf};

use librojo::commands::{serve, ServeOptions};

#[no_mangle]
pub extern "C" fn rojo_serve(path: *const c_char) {
    let path = unsafe { PathBuf::from(CStr::from_ptr(path).to_str().unwrap()) };

    serve(&ServeOptions {
        fuzzy_project_path: path,
        port: None,
    })
    .unwrap();
}
