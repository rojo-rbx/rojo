use hyper::{header::CONTENT_TYPE, Body, Response, StatusCode};
use serde::Serialize;

pub fn msgpack_ok<T: Serialize>(value: T) -> Response<Body> {
    msgpack(value, StatusCode::OK)
}

pub fn msgpack<T: Serialize>(value: T, code: StatusCode) -> Response<Body> {
    let mut serialized = Vec::new();
    let mut serializer = rmp_serde::Serializer::new(&mut serialized)
        .with_human_readable()
        .with_struct_map();

    if let Err(err) = value.serialize(&mut serializer) {
        return Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .header(CONTENT_TYPE, "text/plain")
            .body(Body::from(err.to_string()))
            .unwrap();
    };

    Response::builder()
        .status(code)
        .header(CONTENT_TYPE, "application/msgpack")
        .body(Body::from(serialized))
        .unwrap()
}

pub fn json<T: Serialize>(value: T, code: StatusCode) -> Response<Body> {
    let serialized = match serde_json::to_string(&value) {
        Ok(v) => v,
        Err(err) => {
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header(CONTENT_TYPE, "text/plain")
                .body(Body::from(err.to_string()))
                .unwrap();
        }
    };

    Response::builder()
        .status(code)
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(serialized))
        .unwrap()
}
