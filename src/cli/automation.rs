use std::{
    io::Read,
    net::IpAddr,
    thread,
    time::{Duration, Instant},
};

use anyhow::{anyhow, bail, Context};
use reqwest::{
    blocking::{Client, RequestBuilder, Response},
    header::{ACCEPT, CONTENT_TYPE},
    StatusCode,
};
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::automation::MAX_AUTOMATION_RESULT_BODY_BYTES;
use crate::web::{
    deserialize_msgpack,
    interface::{ErrorResponse, ServerInfoResponse, PROTOCOL_VERSION},
    serialize_msgpack,
};

pub(super) const DEFAULT_ADDRESS: &str = "127.0.0.1";
pub(super) const DEFAULT_PORT: u16 = 34872;
pub(super) const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
pub(super) const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
pub(super) const MSGPACK_CONTENT_TYPE: &str = "application/msgpack";
const MAX_RESPONSE_SIZE_BYTES: usize = MAX_AUTOMATION_RESULT_BODY_BYTES + 64 * 1024;

pub(super) fn build_client(operation: &str) -> anyhow::Result<Client> {
    Client::builder()
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(REQUEST_TIMEOUT)
        .build()
        .with_context(|| format!("Could not create the HTTP client for {operation}"))
}

pub(super) fn server_url(address: IpAddr, port: u16) -> String {
    match address {
        IpAddr::V4(address) => format!("http://{address}:{port}"),
        IpAddr::V6(address) => format!("http://[{address}]:{port}"),
    }
}

pub(super) fn verify_rojo_server(
    client: &Client,
    server_url: &str,
    client_description: &str,
) -> anyhow::Result<()> {
    let response = send_request(
        client
            .get(format!("{server_url}/api/rojo"))
            .header(ACCEPT, MSGPACK_CONTENT_TYPE),
        server_url,
        "checking the server",
    )?;
    let response = buffer_response(response, "checking the server")?;

    if response.status != StatusCode::OK {
        let details = error_details(&response).unwrap_or_default();
        bail!(
            "The HTTP service at {server_url} did not return Rojo server information (HTTP {}){details}.",
            response.status
        );
    }

    if !response.is_msgpack {
        let content_type = response.content_type.as_deref().unwrap_or("missing");
        bail!(
            "The HTTP service at {server_url} returned content type '{content_type}' from /api/rojo, not {MSGPACK_CONTENT_TYPE}; it does not appear to be a compatible Rojo server."
        );
    }

    let info: ServerInfoResponse = deserialize_msgpack(&response.body).map_err(|error| {
        anyhow!(
            "The Rojo server at {server_url} returned malformed MessagePack from /api/rojo: {error}"
        )
    })?;

    if info.protocol_version != PROTOCOL_VERSION {
        bail!(
            "The Rojo server at {server_url} uses protocol version {}, but this {client_description} requires version {}.",
            info.protocol_version,
            PROTOCOL_VERSION
        );
    }

    Ok(())
}

pub(super) fn send_request(
    request: RequestBuilder,
    server_url: &str,
    operation: &str,
) -> anyhow::Result<Response> {
    request.send().map_err(|error| {
        if error.is_connect() {
            anyhow!("Could not connect to the Rojo server at {server_url} while {operation}: {error}")
        } else if error.is_timeout() {
            anyhow!("The request to the Rojo server at {server_url} timed out while {operation}: {error}")
        } else {
            anyhow!("The request to the Rojo server at {server_url} failed while {operation}: {error}")
        }
    })
}

pub(super) struct BufferedResponse {
    pub status: StatusCode,
    pub content_type: Option<String>,
    pub is_msgpack: bool,
    pub body: Vec<u8>,
}

pub(super) fn buffer_response(
    response: Response,
    operation: &str,
) -> anyhow::Result<BufferedResponse> {
    let status = response.status();
    let content_type = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let is_msgpack = content_type
        .as_deref()
        .and_then(|value| value.split(';').next())
        .is_some_and(|value| value.trim().eq_ignore_ascii_case(MSGPACK_CONTENT_TYPE));
    let mut body = Vec::new();
    let mut limited = response.take((MAX_RESPONSE_SIZE_BYTES + 1) as u64);
    limited
        .read_to_end(&mut body)
        .with_context(|| format!("Could not read the server response while {operation}"))?;

    if body.len() > MAX_RESPONSE_SIZE_BYTES {
        bail!(
            "The Rojo server response exceeded the {}-byte client limit while {operation}.",
            MAX_RESPONSE_SIZE_BYTES
        );
    }

    Ok(BufferedResponse {
        status,
        content_type,
        is_msgpack,
        body,
    })
}

pub(super) fn decode_response<T: DeserializeOwned>(
    response: Response,
    expected_status: StatusCode,
    server_url: &str,
    operation: &str,
    status_summary: fn(StatusCode) -> &'static str,
) -> anyhow::Result<T> {
    let response = buffer_response(response, operation)?;
    decode_buffered_response(
        &response,
        expected_status,
        server_url,
        operation,
        status_summary,
    )
}

pub(super) fn get_msgpack<T: DeserializeOwned>(
    client: &Client,
    server_url: &str,
    path: &str,
    expected_status: StatusCode,
    operation: &str,
) -> anyhow::Result<T> {
    let response = send_request(
        client
            .get(format!("{server_url}{path}"))
            .header(ACCEPT, MSGPACK_CONTENT_TYPE),
        server_url,
        operation,
    )?;
    decode_response(
        response,
        expected_status,
        server_url,
        operation,
        automation_http_summary,
    )
}

pub(super) fn post_msgpack<Request: Serialize, Output: DeserializeOwned>(
    client: &Client,
    server_url: &str,
    path: &str,
    request: &Request,
    expected_status: StatusCode,
    operation: &str,
) -> anyhow::Result<Output> {
    let body = serialize_msgpack(request)
        .with_context(|| format!("Could not encode the request while {operation}"))?;
    let response = send_request(
        client
            .post(format!("{server_url}{path}"))
            .header(ACCEPT, MSGPACK_CONTENT_TYPE)
            .header(CONTENT_TYPE, MSGPACK_CONTENT_TYPE)
            .body(body),
        server_url,
        operation,
    )?;
    decode_response(
        response,
        expected_status,
        server_url,
        operation,
        automation_http_summary,
    )
}

pub(super) fn decode_buffered_response<T: DeserializeOwned>(
    response: &BufferedResponse,
    expected_status: StatusCode,
    server_url: &str,
    operation: &str,
    status_summary: fn(StatusCode) -> &'static str,
) -> anyhow::Result<T> {
    if response.status != expected_status {
        return Err(api_http_error(response, operation, status_summary));
    }

    if !response.is_msgpack {
        let content_type = response.content_type.as_deref().unwrap_or("missing");
        bail!(
            "The Rojo server at {server_url} returned content type '{content_type}' while {operation}; expected {MSGPACK_CONTENT_TYPE}."
        );
    }

    deserialize_msgpack(&response.body).map_err(|error| {
        anyhow!(
            "The Rojo server at {server_url} returned malformed MessagePack while {operation}: {error}"
        )
    })
}

pub(super) struct PollOptions<'a> {
    pub server_url: &'a str,
    pub path: &'a str,
    pub timeout: Duration,
    pub interval: Duration,
    pub operation: &'a str,
}

pub(super) fn poll_status<T>(
    client: &Client,
    options: PollOptions<'_>,
    decode: impl Fn(Response) -> anyhow::Result<T>,
    is_terminal: impl Fn(&T) -> anyhow::Result<bool>,
    timeout_error: impl Fn() -> anyhow::Error,
) -> anyhow::Result<T> {
    let started_at = Instant::now();

    loop {
        let elapsed = started_at.elapsed();
        if elapsed >= options.timeout {
            return Err(timeout_error());
        }
        let remaining = options.timeout - elapsed;
        let response = send_request(
            client
                .get(format!("{}{}", options.server_url, options.path))
                .timeout(REQUEST_TIMEOUT.min(remaining))
                .header(ACCEPT, MSGPACK_CONTENT_TYPE),
            options.server_url,
            options.operation,
        )
        .map_err(|error| {
            if started_at.elapsed() >= options.timeout {
                timeout_error()
            } else {
                error
            }
        })?;
        let status = decode(response)?;
        if is_terminal(&status)? {
            return Ok(status);
        }

        let elapsed = started_at.elapsed();
        if elapsed >= options.timeout {
            return Err(timeout_error());
        }
        thread::sleep(options.interval.min(options.timeout - elapsed));
    }
}

fn api_http_error(
    response: &BufferedResponse,
    operation: &str,
    status_summary: fn(StatusCode) -> &'static str,
) -> anyhow::Error {
    let details = error_details(response).unwrap_or_default();
    anyhow!(
        "{} while {operation} (HTTP {}){details}.",
        status_summary(response.status),
        response.status
    )
}

fn error_details(response: &BufferedResponse) -> Option<String> {
    if !response.is_msgpack {
        return None;
    }

    let error: ErrorResponse = deserialize_msgpack(&response.body).ok()?;
    if error.details().is_empty() {
        Some(format!(" [{:?}]", error.kind()))
    } else {
        Some(format!(" [{:?}: {}]", error.kind(), error.details()))
    }
}

pub(super) fn automation_http_summary(status: StatusCode) -> &'static str {
    match status {
        StatusCode::BAD_REQUEST => "Prism automation request was malformed",
        StatusCode::FORBIDDEN => "Prism automation API is not available from this peer",
        StatusCode::NOT_FOUND => "Prism automation job disappeared or expired",
        StatusCode::CONFLICT => "Prism automation request has a state or session conflict",
        StatusCode::PAYLOAD_TOO_LARGE => "Prism automation request or result is too large",
        StatusCode::TOO_MANY_REQUESTS => "Prism automation queue is full",
        StatusCode::INTERNAL_SERVER_ERROR => "Prism automation server reported an internal error",
        _ => "Prism automation server returned an unexpected HTTP status",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::web::serialize_msgpack;

    fn summary(_status: StatusCode) -> &'static str {
        "automation request failed"
    }

    #[test]
    fn constructs_ipv4_and_ipv6_server_urls() {
        assert_eq!(
            server_url("127.0.0.1".parse().unwrap(), 34872),
            "http://127.0.0.1:34872"
        );
        assert_eq!(
            server_url("::1".parse().unwrap(), 34872),
            "http://[::1]:34872"
        );
    }

    #[test]
    fn decodes_common_messagepack_error_envelopes() {
        let response = BufferedResponse {
            status: StatusCode::CONFLICT,
            content_type: Some(MSGPACK_CONTENT_TYPE.to_owned()),
            is_msgpack: true,
            body: serialize_msgpack(ErrorResponse::conflict("duplicate session")).unwrap(),
        };
        let error = decode_buffered_response::<serde_json::Value>(
            &response,
            StatusCode::OK,
            "http://127.0.0.1:34872",
            "checking automation status",
            summary,
        )
        .unwrap_err();

        assert!(error.to_string().contains("Conflict: duplicate session"));
    }
}
