use std::fs::{create_dir, copy};
use std::path::Path;
use std::io;

use rouille::Request;

use walkdir::WalkDir;

use librojo::web::Server;

pub trait HttpTestUtil {
    fn get_string(&self, url: &str) -> String;
}

impl HttpTestUtil for Server {
    fn get_string(&self, url: &str) -> String {
        let info_request = Request::fake_http("GET", url, vec![], vec![]);
        let response = self.handle_request(&info_request);

        assert_eq!(response.status_code, 200);

        let (mut reader, _) = response.data.into_reader_and_size();
        let mut body = String::new();
        reader.read_to_string(&mut body).unwrap();

        body
    }
}

pub fn copy_recursive(from: &Path, to: &Path) -> io::Result<()> {
    for entry in WalkDir::new(from) {
        let entry = entry?;
        let path = entry.path();
        let new_path = to.join(path.strip_prefix(from).unwrap());

        let file_type = entry.file_type();

        if file_type.is_dir() {
            match create_dir(new_path) {
                Ok(_) => {},
                Err(err) => match err.kind() {
                    io::ErrorKind::AlreadyExists => {},
                    _ => panic!(err),
                }
            }
        } else if file_type.is_file() {
            copy(path, new_path)?;
        } else {
            unimplemented!("no symlinks please");
        }
    }

    Ok(())
}
