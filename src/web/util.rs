use hyper::{header::CONTENT_TYPE, Body, Response, StatusCode};
use serde::{Deserialize, Serialize};

/// Builds an HTTP response, falling back to an empty `500` response (rather than
/// panicking) if the response could not be constructed. With constant headers
/// and a valid status code this never actually fails, but routing every
/// response through here means a malformed response can never crash the server.
pub fn response(
    code: StatusCode,
    content_type: &'static str,
    body: impl Into<Body>,
) -> Response<Body> {
    Response::builder()
        .status(code)
        .header(CONTENT_TYPE, content_type)
        .body(body.into())
        .unwrap_or_else(|err| {
            log::error!("Failed to build HTTP response: {}", err);
            let mut fallback = Response::new(Body::empty());
            *fallback.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
            fallback
        })
}

pub fn msgpack_ok<T: Serialize>(value: T) -> Response<Body> {
    msgpack(value, StatusCode::OK)
}

pub fn msgpack<T: Serialize>(value: T, code: StatusCode) -> Response<Body> {
    let mut serialized = Vec::new();
    let mut serializer = rmp_serde::Serializer::new(&mut serialized)
        .with_human_readable()
        .with_struct_map();

    if let Err(err) = value.serialize(&mut serializer) {
        return response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "text/plain",
            err.to_string(),
        );
    };

    response(code, "application/msgpack", serialized)
}

pub fn serialize_msgpack<T: Serialize>(value: T) -> anyhow::Result<Vec<u8>> {
    let mut serialized = Vec::new();
    let mut serializer = rmp_serde::Serializer::new(&mut serialized)
        .with_human_readable()
        .with_struct_map();

    value.serialize(&mut serializer)?;

    Ok(serialized)
}

pub fn deserialize_msgpack<'a, T: Deserialize<'a>>(
    input: &'a [u8],
) -> Result<T, rmp_serde::decode::Error> {
    let mut deserializer = rmp_serde::Deserializer::new(input).with_human_readable();

    T::deserialize(&mut deserializer)
}

pub fn json<T: Serialize>(value: T, code: StatusCode) -> Response<Body> {
    let serialized = match serde_json::to_string(&value) {
        Ok(v) => v,
        Err(err) => {
            return response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "text/plain",
                err.to_string(),
            );
        }
    };

    response(code, "application/json", serialized)
}
