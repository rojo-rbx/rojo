use futures::{future, Future};
use hyper::{header::CONTENT_TYPE, Body, Response, StatusCode};
use serde::Serialize;

fn response_json<T: Serialize>(value: T, code: StatusCode) -> Response<Body> {
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

pub fn json<T: Serialize>(
    value: T,
    code: StatusCode,
) -> Box<dyn Future<Item = hyper::Response<hyper::Body>, Error = hyper::Error> + Send> {
    Box::new(future::ok(response_json(value, code)))
}

pub fn json_ok<T: Serialize>(
    value: T,
) -> Box<dyn Future<Item = hyper::Response<hyper::Body>, Error = hyper::Error> + Send> {
    json(value, StatusCode::OK)
}
