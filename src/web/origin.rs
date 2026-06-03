//! Host/Origin validation used to defend the local Rojo server against DNS
//! rebinding attacks.
//!
//! When Rojo is bound to a loopback address (the default), a malicious web page
//! the developer visits cannot read the API responses directly because of the
//! browser Same-Origin Policy. However, a DNS rebinding attack can defeat that:
//! the page points its own hostname at `127.0.0.1` after loading, so the browser
//! treats requests to the Rojo server as same-origin. Validating that the `Host`
//! (and, if present, `Origin`) header refers to an address we recognize blocks
//! this, because the rebound request still carries the attacker's hostname, which
//! is a domain name rather than one of the IP literals we accept.
//!
//! Enforcement covers two kinds of bind:
//!
//! * Loopback (the default): only `localhost` and loopback literals are
//!   accepted.
//! * A specific private/LAN address: `localhost`, loopback literals, and that exact bind
//!   IP are accepted. Because the defense works by rejecting any `Host` that
//!   isn't a recognized IP literal, clients must connect to a private bind by
//!   IP. A hostname (e.g. `mypc.local`) is indistinguishable from an attacker
//!   domain and is rejected.
//!
//! Enforcement is disabled for unspecified (`0.0.0.0`/`::`) and public binds: the
//! user has asked for broad, possibly-public exposure where arbitrary hostnames
//! may legitimately resolve to the server, so we can't build a meaningful
//! allowlist. Those binds get a startup warning instead. Two consequences worth
//! being explicit about: such a bind has no rebinding protection even for a
//! browser on the same machine; and even on a protected private bind this check
//! does nothing against a hostile peer already on the LAN, who can reach the
//! unauthenticated API directly. Both are the network-exposure risk the startup
//! warning addresses, not something the `Host` check is meant to cover.
//!
//! The allowlist can be widened with extra hosts (the `--allowed-hosts` CLI
//! option or a project's `serveAllowedHosts`), for example a hostname like
//! `mypc.lan` for reaching a network-exposed server by name. Listing any extra
//! host also turns enforcement back on for an unspecified or public bind that
//! would otherwise disable it, restricting that bind to localhost, the bind IP,
//! and the listed hosts.

use std::net::IpAddr;

use hyper::{
    header::{HOST, ORIGIN},
    http::uri::{Authority, Uri},
    Body, Request, Response, StatusCode,
};

use crate::web::util::response;

/// The set of `Host`/`Origin` values accepted while enforcement is active.
#[derive(Debug, Clone)]
pub struct AllowedHosts {
    port: u16,
    /// The bind IP accepted in addition to `localhost`/loopback, set only for a
    /// private (LAN) bind or a specific public bind that has extra hosts. `None`
    /// for a loopback or unspecified bind, which has no single address to add.
    bind_ip: Option<IpAddr>,
    /// Extra `Host`/`Origin` values the user explicitly allowed, via the
    /// `--allowed-hosts` option or a project's `serveAllowedHosts`. These are
    /// hostnames such as `mypc.lan`, or IP literals, accepted in addition to
    /// localhost and the bind IP. Each entry is already passed through
    /// [`normalize_host`].
    extra_hosts: Vec<String>,
}

impl AllowedHosts {
    /// Returns whether the given host and optional port are allowed. The host is
    /// accepted if it is `localhost`, a loopback IP literal, (on a private or
    /// specific public bind) the exact bind IP, or one of the explicitly allowed
    /// extra hosts. A request with no explicit port is accepted (the host
    /// already has to be one we recognize).
    fn allows(&self, host: &str, port: Option<u16>) -> bool {
        let host = normalize_host(host);
        let host_ok = host.eq_ignore_ascii_case("localhost")
            || host
                .parse::<IpAddr>()
                .ok()
                .map(canonical)
                .is_some_and(|ip| ip.is_loopback() || self.bind_ip == Some(ip))
            || self.allows_extra(host);

        host_ok && port.is_none_or(|port| port == self.port)
    }

    /// Returns whether `host` matches one of the explicitly allowed extra hosts.
    /// Entries that are IP literals are compared as addresses, so equivalent
    /// forms (such as an IPv4-mapped IPv6 literal) match; everything else is
    /// compared as a case-insensitive hostname.
    fn allows_extra(&self, host: &str) -> bool {
        let host_ip = host.parse::<IpAddr>().ok().map(canonical);
        self.extra_hosts.iter().any(|allowed| {
            match (host_ip, allowed.parse::<IpAddr>().map(canonical)) {
                (Some(host_ip), Ok(allowed_ip)) => host_ip == allowed_ip,
                _ => host.eq_ignore_ascii_case(allowed),
            }
        })
    }
}

/// Builds the allowlist for a given bind address and any extra allowed hosts.
/// Returns `None` (validation disabled) when bound to an unspecified
/// (`0.0.0.0`/`::`) or public address and no extra hosts were given, where
/// arbitrary hostnames may legitimately resolve to the server. Listing extra
/// hosts keeps validation on even for those binds.
pub fn allowed_hosts(bind: IpAddr, port: u16, extra: &[String]) -> Option<AllowedHosts> {
    let extra_hosts: Vec<String> = extra
        .iter()
        .map(|host| normalize_host(host.trim()).to_owned())
        .filter(|host| !host.is_empty())
        .collect();

    if bind.is_loopback() {
        Some(AllowedHosts {
            port,
            bind_ip: None,
            extra_hosts,
        })
    } else if is_private_bind(bind) {
        Some(AllowedHosts {
            port,
            bind_ip: Some(canonical(bind)),
            extra_hosts,
        })
    } else if !extra_hosts.is_empty() {
        // The bind is unspecified or public, where validation is normally
        // disabled. By listing explicit hosts the user has opted back into it,
        // so we accept localhost, the bind IP (when it is a specific address),
        // and those hosts. An unspecified bind (`0.0.0.0`/`::`) has no single
        // address to add.
        let bind_ip = (!bind.is_unspecified()).then(|| canonical(bind));
        Some(AllowedHosts {
            port,
            bind_ip,
            extra_hosts,
        })
    } else {
        None
    }
}

/// Collapses an IPv4-mapped IPv6 address (`::ffff:192.168.0.1`) to its IPv4 form
/// so it classifies and compares consistently with the bare IPv4 address. Shared
/// with the `/api/open` peer check so it recognizes a mapped loopback peer too.
pub(crate) fn canonical(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V6(v6) => v6.to_ipv4_mapped().map(IpAddr::V4).unwrap_or(ip),
        v4 => v4,
    }
}

/// Returns whether a bind address is a specific private/link-local address, the
/// case where enforcement stays on with the bind IP added to the allowlist.
/// Unspecified, loopback, and public addresses are excluded (loopback is handled
/// separately by [`allowed_hosts`]).
fn is_private_bind(ip: IpAddr) -> bool {
    match canonical(ip) {
        IpAddr::V4(v4) => v4.is_private() || v4.is_link_local(),
        IpAddr::V6(v6) => {
            let first = v6.segments()[0];
            // Unique local (fc00::/7) or link-local (fe80::/10). The std methods
            // for these are still nightly-only, so test the prefix directly.
            (first & 0xfe00) == 0xfc00 || (first & 0xffc0) == 0xfe80
        }
    }
}

/// Validates the `Host` and `Origin` headers of an incoming request against the
/// allowlist. Returns `Some` with a ready-to-send `404` response when the
/// request should be rejected, or `None` when it is allowed to proceed. When
/// `allowed` is `None` (unspecified or public bind) every request is accepted.
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

/// Normalizes a host literal for parsing/comparison: strips the surrounding
/// brackets from an IPv6 literal (e.g. `[::1]`) and drops any IPv6 zone id (e.g.
/// `fe80::1%eth0`), which `Ipv6Addr::from_str` would otherwise reject.
fn normalize_host(host: &str) -> &str {
    let host = host
        .strip_prefix('[')
        .and_then(|host| host.strip_suffix(']'))
        .unwrap_or(host);

    host.split('%').next().unwrap_or(host)
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

/// Builds the response sent when a request fails Host/Origin validation. It is a
/// generic `404` with no Rojo-identifying body: a rejected request may be a
/// prober (or a DNS-rebound page's same-origin script, which could read the
/// body), so we reveal nothing rather than confirming a Rojo server is here.
fn reject() -> Response<Body> {
    response(StatusCode::NOT_FOUND, "text/plain", "Not Found")
}

#[cfg(test)]
mod test {
    use super::*;

    use std::net::{Ipv4Addr, Ipv6Addr};

    const PORT: u16 = 34872;

    fn loopback_allowlist() -> Option<AllowedHosts> {
        allowed_hosts(IpAddr::V4(Ipv4Addr::LOCALHOST), PORT, &[])
    }

    /// An allowlist for a server bound to a specific private (LAN) address.
    fn private_allowlist() -> Option<AllowedHosts> {
        allowed_hosts(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 5)), PORT, &[])
    }

    /// An allowlist for `bind` with the given extra allowed hosts.
    fn allowlist_with_hosts(bind: IpAddr, hosts: &[&str]) -> Option<AllowedHosts> {
        let hosts: Vec<String> = hosts.iter().map(|host| host.to_string()).collect();
        allowed_hosts(bind, PORT, &hosts)
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
        assert!(allowed_hosts(IpAddr::V4(Ipv4Addr::LOCALHOST), PORT, &[]).is_some());
        assert!(allowed_hosts(IpAddr::V6(Ipv6Addr::LOCALHOST), PORT, &[]).is_some());
    }

    #[test]
    fn private_bind_enables_enforcement() {
        for bind in [
            IpAddr::V4(Ipv4Addr::new(192, 168, 1, 5)), // 192.168.0.0/16
            IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)),    // 10.0.0.0/8
            IpAddr::V4(Ipv4Addr::new(172, 16, 0, 1)),  // 172.16.0.0/12
            IpAddr::V4(Ipv4Addr::new(169, 254, 0, 1)), // link-local
            IpAddr::V6(Ipv6Addr::new(0xfc00, 0, 0, 0, 0, 0, 0, 1)), // unique local
            IpAddr::V6(Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1)), // link-local
        ] {
            assert!(
                allowed_hosts(bind, PORT, &[]).is_some(),
                "private bind {bind} should enable enforcement"
            );
        }
    }

    #[test]
    fn unspecified_or_public_bind_disables_enforcement() {
        for bind in [
            IpAddr::V4(Ipv4Addr::UNSPECIFIED),     // 0.0.0.0
            IpAddr::V6(Ipv6Addr::UNSPECIFIED),     // ::
            IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)), // public
            IpAddr::V6(Ipv6Addr::new(0x2001, 0x4860, 0, 0, 0, 0, 0, 0x8888)), // public
        ] {
            assert!(
                allowed_hosts(bind, PORT, &[]).is_none(),
                "bind {bind} should disable enforcement"
            );
        }
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
    fn private_bind_accepts_local_and_bind_ip_hosts() {
        let allowed = private_allowlist();
        for host in [
            format!("192.168.1.5:{PORT}"),
            format!("localhost:{PORT}"),
            format!("127.0.0.1:{PORT}"),
            format!("[::1]:{PORT}"),
            "192.168.1.5".to_owned(),
        ] {
            let request = request_with(&[("host", &host)]);
            assert!(
                check_request_origin(&request, allowed.as_ref()).is_none(),
                "host {host} should be allowed on a private bind"
            );
        }
    }

    #[test]
    fn private_bind_rejects_other_hosts() {
        let allowed = private_allowlist();
        for host in [
            "evil.com",    // a rebound attacker domain
            "192.168.1.6", // a different private IP
            "8.8.8.8",     // a public IP
        ] {
            let request = request_with(&[("host", host)]);
            assert!(
                check_request_origin(&request, allowed.as_ref()).is_some(),
                "host {host} should be rejected on a private bind"
            );
        }
    }

    #[test]
    fn private_bind_keeps_origin_strict() {
        // A different private IP as Origin must be rejected even though the Host
        // is the valid bind IP: the Origin check is not widened to arbitrary
        // private addresses.
        let allowed = private_allowlist();
        let request = request_with(&[
            ("host", &format!("192.168.1.5:{PORT}")),
            ("origin", &format!("http://192.168.1.6:{PORT}")),
        ]);
        assert!(check_request_origin(&request, allowed.as_ref()).is_some());
    }

    #[test]
    fn private_bind_accepts_bind_ip_origin() {
        let allowed = private_allowlist();
        let request = request_with(&[
            ("host", &format!("192.168.1.5:{PORT}")),
            ("origin", &format!("http://192.168.1.5:{PORT}")),
        ]);
        assert!(check_request_origin(&request, allowed.as_ref()).is_none());
    }

    #[test]
    fn accepts_ipv4_mapped_bind_ip() {
        // `::ffff:192.168.1.5` is the IPv4-mapped form of the bind IP and must
        // be treated as equal to it.
        let allowed = private_allowlist();
        let request = request_with(&[("host", &format!("[::ffff:192.168.1.5]:{PORT}"))]);
        assert!(check_request_origin(&request, allowed.as_ref()).is_none());
    }

    #[test]
    fn canonical_collapses_ipv4_mapped_loopback() {
        // The `/api/open` peer gate relies on this: an IPv4 loopback client
        // reaching a dual-stack (`::`) bind arrives as `::ffff:127.0.0.1`, which
        // is only recognized as loopback after canonicalization.
        let mapped: IpAddr = "::ffff:127.0.0.1".parse().unwrap();
        assert!(!mapped.is_loopback());
        assert!(canonical(mapped).is_loopback());
    }

    #[test]
    fn normalize_host_strips_brackets_and_zone_id() {
        // Brackets and an IPv6 zone id must be removed so the result parses as an
        // `IpAddr` (`Ipv6Addr::from_str` rejects zone ids).
        assert_eq!(normalize_host("[::1]"), "::1");
        assert_eq!(normalize_host("[fe80::1%eth0]"), "fe80::1");
        assert_eq!(normalize_host("localhost"), "localhost");
        assert!(normalize_host("[fe80::1%eth0]").parse::<IpAddr>().is_ok());
    }

    #[test]
    fn allowed_hosts_extend_the_allowlist() {
        // A hostname listed as an allowed host is accepted on a loopback bind,
        // with the usual port check still applied; an unlisted host is not.
        let allowed = allowlist_with_hosts(IpAddr::V4(Ipv4Addr::LOCALHOST), &["mypc.lan"]);

        let ok = request_with(&[("host", &format!("mypc.lan:{PORT}"))]);
        assert!(check_request_origin(&ok, allowed.as_ref()).is_none());

        let wrong_port = request_with(&[("host", "mypc.lan:1234")]);
        assert!(check_request_origin(&wrong_port, allowed.as_ref()).is_some());

        let other = request_with(&[("host", &format!("other.lan:{PORT}"))]);
        assert!(check_request_origin(&other, allowed.as_ref()).is_some());
    }

    #[test]
    fn allowed_hosts_apply_to_origin() {
        let allowed = allowlist_with_hosts(IpAddr::V4(Ipv4Addr::LOCALHOST), &["mypc.lan"]);

        let ok = request_with(&[
            ("host", &format!("mypc.lan:{PORT}")),
            ("origin", &format!("http://mypc.lan:{PORT}")),
        ]);
        assert!(check_request_origin(&ok, allowed.as_ref()).is_none());

        let foreign_origin = request_with(&[
            ("host", &format!("mypc.lan:{PORT}")),
            ("origin", &format!("http://evil.com:{PORT}")),
        ]);
        assert!(check_request_origin(&foreign_origin, allowed.as_ref()).is_some());
    }

    #[test]
    fn allowed_hosts_enable_enforcement_on_exposed_bind() {
        // Binding to 0.0.0.0 normally disables validation, but listing a host
        // turns it back on: localhost and the listed host are accepted while
        // everything else is rejected.
        let allowed = allowlist_with_hosts(IpAddr::V4(Ipv4Addr::UNSPECIFIED), &["mypc.lan"]);
        assert!(allowed.is_some());

        for host in [format!("mypc.lan:{PORT}"), format!("localhost:{PORT}")] {
            let request = request_with(&[("host", &host)]);
            assert!(
                check_request_origin(&request, allowed.as_ref()).is_none(),
                "host {host} should be allowed"
            );
        }

        let evil = request_with(&[("host", "evil.com")]);
        assert!(check_request_origin(&evil, allowed.as_ref()).is_some());
    }

    #[test]
    fn allowed_hosts_on_public_bind_accept_the_bind_ip() {
        // A specific public bind with an allowed host accepts localhost, the
        // listed host, and the bind IP itself, but nothing else. (203.0.113.0/24
        // is the TEST-NET-3 documentation range, treated here as a public IP.)
        let bind = IpAddr::V4(Ipv4Addr::new(203, 0, 113, 5));
        let allowed = allowlist_with_hosts(bind, &["mypc.lan"]);

        for host in [
            format!("203.0.113.5:{PORT}"),
            format!("mypc.lan:{PORT}"),
            format!("localhost:{PORT}"),
        ] {
            let request = request_with(&[("host", &host)]);
            assert!(
                check_request_origin(&request, allowed.as_ref()).is_none(),
                "host {host} should be allowed"
            );
        }

        let evil = request_with(&[("host", "evil.com")]);
        assert!(check_request_origin(&evil, allowed.as_ref()).is_some());
    }

    #[test]
    fn disabled_allowlist_accepts_foreign_host() {
        for bind in [
            IpAddr::V4(Ipv4Addr::UNSPECIFIED),
            IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)),
        ] {
            let allowed = allowed_hosts(bind, PORT, &[]);
            let request = request_with(&[("host", "evil.com")]);
            assert!(
                check_request_origin(&request, allowed.as_ref()).is_none(),
                "disabled allowlist for bind {bind} should accept any host"
            );
        }
    }
}
