use hyper::{header::CONTENT_TYPE, Body, Response, StatusCode};
use serde::Serialize;

pub fn response_json<T: Serialize>(value: T) -> Response<Body> {
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
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(serialized))
        .unwrap()
}
