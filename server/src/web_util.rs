use std::io::Read;

use rouille;
use serde;
use serde_json;

static MAX_BODY_SIZE: usize = 100 * 1024 * 1024; // 100 MiB

/// Pulls text that may be JSON out of a Rouille Request object.
///
/// Doesn't do any actual parsing -- all this method does is verify the content
/// type of the request and read the request's body.
fn read_json_text(request: &rouille::Request) -> Option<String> {
    // Bail out if the request body isn't marked as JSON
    let content_type = request.header("Content-Type")?;

    if !content_type.starts_with("application/json") {
        return None;
    }

    let body = request.data()?;

    // Allocate a buffer and read up to MAX_BODY_SIZE+1 bytes into it.
    let mut out = Vec::new();
    body.take(MAX_BODY_SIZE.saturating_add(1) as u64).read_to_end(&mut out).ok()?;

    // If the body was too big (MAX_BODY_SIZE+1), we abort instead of trying to
    // process it.
    if out.len() > MAX_BODY_SIZE {
        return None;
    }

    String::from_utf8(out).ok()
}

/// Reads the body out of a Rouille Request and attempts to turn it into JSON.
pub fn read_json<T>(request: &rouille::Request) -> Option<T>
where
    T: serde::de::DeserializeOwned,
{
    let body = read_json_text(&request)?;
    serde_json::from_str(&body).ok()?
}
