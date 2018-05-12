use std::io::Read;

use rouille;
use serde;
use serde_json;

static MAX_BODY_SIZE: usize = 100 * 1024 * 1024; // 100 MiB

pub fn json_response<T: serde::Serialize>(value: T) -> rouille::Response {
    let data = serde_json::to_string(&value).unwrap();
    rouille::Response::from_data("application/json", data)
}

/// Pulls text that may be JSON out of a Rouille Request object.
///
/// Doesn't do any actual parsing -- all this method does is verify the content
/// type of the request and read the request's body.
fn read_json_text(request: &rouille::Request) -> Option<String> {
    // Bail out if the request body isn't marked as JSON
    match request.header("Content-Type") {
        Some(header) => if !header.starts_with("application/json") {
            return None;
        },
        None => return None,
    }

    let body = match request.data() {
        Some(v) => v,
        None => return None,
    };

    // Allocate a buffer and read up to MAX_BODY_SIZE+1 bytes into it.
    let mut out = Vec::new();
    match body.take(MAX_BODY_SIZE.saturating_add(1) as u64).read_to_end(&mut out) {
        Ok(_) => {},
        Err(_) => return None,
    }

    // If the body was too big (MAX_BODY_SIZE+1), we abort instead of trying to
    // process it.
    if out.len() > MAX_BODY_SIZE {
        return None;
    }

    let parsed = match String::from_utf8(out) {
        Ok(v) => v,
        Err(_) => return None,
    };

    Some(parsed)
}

/// Reads the body out of a Rouille Request and attempts to turn it into JSON.
pub fn read_json<T>(request: &rouille::Request) -> Option<T>
where
    T: serde::de::DeserializeOwned,
{
    let body = match read_json_text(&request) {
        Some(v) => v,
        None => return None,
    };

    let parsed = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(_) => return None,
    };

    // TODO: Change return type to some sort of Result

    Some(parsed)
}
