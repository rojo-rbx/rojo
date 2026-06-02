//! Host/Origin validation used to defend the local Rojo server against DNS
//! rebinding attacks.
//!
//! When Rojo is bound to a loopback address (the default), a malicious web page
//! the developer visits cannot read the API responses directly because of the
//! browser Same-Origin Policy. However, a DNS rebinding attack can defeat that:
//! the page points its own hostname at `127.0.0.1` after loading, so the browser
//! treats requests to the Rojo server as same-origin. Validating that the `Host`
//! (and, if present, `Origin`) header refers to a local address blocks this,
//! because the rebound request still carries the attacker's hostname.
//!
//! Validation is only enforced for loopback binds. If the user has explicitly
//! exposed the server on a non-loopback address we can't know which hostnames
//! legitimately resolve to it, so enforcement is disabled (they are warned at
//! startup instead). One consequence worth being explicit about: an exposed bind
//! therefore has *no* rebinding protection, even for a browser on the same
//! machine, because the `Host` check that would catch it is turned off. That is
//! an accepted trade-off: exposing the unauthenticated API to the network is
//! already the dominant risk, which the startup warning addresses.

use std::net::IpAddr;

use hyper::{
    header::{HOST, ORIGIN},
    http::uri::{Authority, Uri},
    Body, Request, Response, StatusCode,
};

use crate::web::{interface::ErrorResponse, util::msgpack};

/// The set of `Host`/`Origin` values accepted while the server is bound to a
/// loopback address.
#[derive(Debug, Clone)]
pub struct AllowedHosts {
    port: u16,
}

impl AllowedHosts {
    /// Hostnames considered local, compared case-insensitively against the host
    /// portion of the `Host`/`Origin` header (IPv6 brackets stripped first).
    const LOCAL_HOSTS: [&'static str; 3] = ["localhost", "127.0.0.1", "::1"];

    /// Returns whether the given host and optional port are allowed. A request
    /// with no explicit port is accepted (the host already has to be local).
    fn allows(&self, host: &str, port: Option<u16>) -> bool {
        let host = normalize_host(host);
        let host_is_local = Self::LOCAL_HOSTS
            .iter()
            .any(|local| host.eq_ignore_ascii_case(local));

        host_is_local && port.is_none_or(|port| port == self.port)
    }
}

/// Builds the allowlist for a given bind address. Returns `None` (validation
/// disabled) when the server is bound to a non-loopback address.
pub fn allowed_hosts(bind: IpAddr, port: u16) -> Option<AllowedHosts> {
    if bind.is_loopback() {
        Some(AllowedHosts { port })
    } else {
        None
    }
}

/// Validates the `Host` and `Origin` headers of an incoming request against the
/// allowlist. Returns `Some` with a ready-to-send `403` response when the
/// request should be rejected, or `None` when it is allowed to proceed. When
/// `allowed` is `None` (non-loopback bind) every request is accepted.
pub fn check_request_origin(
    request: &Request<Body>,
    allowed: Option<&AllowedHosts>,
) -> Option<Response<Body>> {
    let allowed = allowed?;

    // The Host header is mandatory and must refer to a local address.
    let host_ok = request
        .headers()
        .get(HOST)
        .and_then(|value| value.to_str().ok())
        .and_then(parse_authority)
        .is_some_and(|(host, port)| allowed.allows(&host, port));

    if !host_ok {
        return Some(reject());
    }

    // The Origin header is optional: non-browser clients such as the Roblox
    // plugin never send it. When it is present (i.e. a browser made the request)
    // it must also be local, which rejects a rebound page whose origin is still
    // its own non-local hostname.
    if let Some(origin) = request.headers().get(ORIGIN) {
        let origin_ok = origin
            .to_str()
            .ok()
            .and_then(parse_origin)
            .is_some_and(|(host, port)| allowed.allows(&host, port));

        if !origin_ok {
            return Some(reject());
        }
    }

    None
}

/// Strips the surrounding brackets from an IPv6 host literal (e.g. `[::1]`), so
/// it can be compared against the unbracketed names in `LOCAL_HOSTS`.
fn normalize_host(host: &str) -> &str {
    host.strip_prefix('[')
        .and_then(|host| host.strip_suffix(']'))
        .unwrap_or(host)
}

/// Parses a `Host` header value (an authority such as `localhost:34872`) into
/// its host and optional port. Returns `None` if the authority carries userinfo
/// (e.g. `evil.com@localhost`), so a value whose host looks local only after the
/// userinfo is stripped can never sneak past the allowlist.
fn parse_authority(value: &str) -> Option<(String, Option<u16>)> {
    let authority: Authority = value.parse().ok()?;
    reject_userinfo(&authority)?;
    Some((authority.host().to_owned(), authority.port_u16()))
}

/// Parses an `Origin` header value (an absolute URI such as
/// `http://localhost:34872`) into its host and optional port. Returns `None` for
/// origins without a host, such as the opaque `null` origin, or for origins whose
/// authority carries userinfo (see [`parse_authority`]).
fn parse_origin(value: &str) -> Option<(String, Option<u16>)> {
    let uri: Uri = value.parse().ok()?;
    reject_userinfo(uri.authority()?)?;
    Some((uri.host()?.to_owned(), uri.port_u16()))
}

/// Returns `None` (rejecting the value) when an authority contains a userinfo
/// component, identified by the `@` separator. A bare host or `host:port` never
/// contains `@`, so this only fires on `userinfo@host` forms.
fn reject_userinfo(authority: &Authority) -> Option<()> {
    if authority.as_str().contains('@') {
        None
    } else {
        Some(())
    }
}

fn reject() -> Response<Body> {
    msgpack(
        ErrorResponse::forbidden(
            "Request rejected because its Host or Origin header is not an allowed local \
             address. This protects the Rojo server from DNS rebinding attacks.",
        ),
        StatusCode::FORBIDDEN,
    )
}

#[cfg(test)]
mod test {
    use super::*;

    use std::net::{Ipv4Addr, Ipv6Addr};

    const PORT: u16 = 34872;

    fn loopback_allowlist() -> Option<AllowedHosts> {
        allowed_hosts(IpAddr::V4(Ipv4Addr::LOCALHOST), PORT)
    }

    fn request_with(headers: &[(&'static str, &str)]) -> Request<Body> {
        let mut builder = Request::builder().uri("/api/rojo");
        for (name, value) in headers {
            builder = builder.header(*name, *value);
        }
        builder.body(Body::empty()).unwrap()
    }

    #[test]
    fn loopback_bind_enables_enforcement() {
        assert!(allowed_hosts(IpAddr::V4(Ipv4Addr::LOCALHOST), PORT).is_some());
        assert!(allowed_hosts(IpAddr::V6(Ipv6Addr::LOCALHOST), PORT).is_some());
    }

    #[test]
    fn non_loopback_bind_disables_enforcement() {
        assert!(allowed_hosts(IpAddr::V4(Ipv4Addr::UNSPECIFIED), PORT).is_none());
        assert!(allowed_hosts(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 5)), PORT).is_none());
    }

    #[test]
    fn accepts_local_hosts() {
        let allowed = loopback_allowlist();
        for host in [
            format!("localhost:{PORT}"),
            format!("127.0.0.1:{PORT}"),
            format!("[::1]:{PORT}"),
            "localhost".to_owned(),
        ] {
            let request = request_with(&[("host", &host)]);
            assert!(
                check_request_origin(&request, allowed.as_ref()).is_none(),
                "host {host} should be allowed"
            );
        }
    }

    #[test]
    fn rejects_foreign_host() {
        let allowed = loopback_allowlist();
        let request = request_with(&[("host", "evil.com")]);
        assert!(check_request_origin(&request, allowed.as_ref()).is_some());
    }

    #[test]
    fn rejects_wrong_port() {
        let allowed = loopback_allowlist();
        let request = request_with(&[("host", "localhost:1234")]);
        assert!(check_request_origin(&request, allowed.as_ref()).is_some());
    }

    #[test]
    fn rejects_missing_host() {
        let allowed = loopback_allowlist();
        let request = request_with(&[]);
        assert!(check_request_origin(&request, allowed.as_ref()).is_some());
    }

    #[test]
    fn rejects_host_with_userinfo() {
        let allowed = loopback_allowlist();
        // The host parses to `localhost`, but the userinfo prefix must keep it
        // from being treated as a local address.
        let request = request_with(&[("host", &format!("evil.com@localhost:{PORT}"))]);
        assert!(check_request_origin(&request, allowed.as_ref()).is_some());
    }

    #[test]
    fn rejects_origin_with_userinfo() {
        let allowed = loopback_allowlist();
        let request = request_with(&[
            ("host", &format!("localhost:{PORT}")),
            ("origin", &format!("http://evil.com@localhost:{PORT}")),
        ]);
        assert!(check_request_origin(&request, allowed.as_ref()).is_some());
    }

    #[test]
    fn rejects_foreign_origin_even_with_local_host() {
        let allowed = loopback_allowlist();
        let request = request_with(&[
            ("host", &format!("localhost:{PORT}")),
            ("origin", &format!("http://evil.com:{PORT}")),
        ]);
        assert!(check_request_origin(&request, allowed.as_ref()).is_some());
    }

    #[test]
    fn rejects_null_origin() {
        let allowed = loopback_allowlist();
        let request = request_with(&[("host", &format!("localhost:{PORT}")), ("origin", "null")]);
        assert!(check_request_origin(&request, allowed.as_ref()).is_some());
    }

    #[test]
    fn accepts_local_origin() {
        let allowed = loopback_allowlist();
        let request = request_with(&[
            ("host", &format!("localhost:{PORT}")),
            ("origin", &format!("http://localhost:{PORT}")),
        ]);
        assert!(check_request_origin(&request, allowed.as_ref()).is_none());
    }

    #[test]
    fn disabled_allowlist_accepts_foreign_host() {
        let allowed = allowed_hosts(IpAddr::V4(Ipv4Addr::UNSPECIFIED), PORT);
        let request = request_with(&[("host", "evil.com")]);
        assert!(check_request_origin(&request, allowed.as_ref()).is_none());
    }
}
