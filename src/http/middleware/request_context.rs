//! Trusted-proxy-aware client IP and request scheme for rate limiting and HTTPS checks.
use axum::{
    body::Body,
    extract::{ConnectInfo, State},
    http::Request,
    middleware::Next,
    response::Response,
};
use ipnet::IpNet;
use std::net::{IpAddr, SocketAddr};

use crate::AppState;

/// How the original request was served (from trusted forwarded headers only).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResolvedScheme {
    Http,
    Https,
    /// Direct connection or untrusted peer — do not enforce HTTPS from headers.
    Unknown,
}

/// Populated for every request after this middleware runs.
#[derive(Clone, Debug)]
pub struct RequestPeer {
    pub client_ip: IpAddr,
    pub scheme: ResolvedScheme,
}

fn peer_in_trusted_proxies(addr: &SocketAddr, cidrs: &[IpNet]) -> bool {
    cidrs.iter().any(|c| c.contains(&addr.ip()))
}

/// Walk `X-Forwarded-For` from the right (the only end a trusted proxy
/// controls), skipping our own trusted proxies, and take the first hop that
/// isn't one of them. Taking the leftmost entry would let clients spoof their
/// IP — and bypass IP rate limits — by sending a forged XFF header.
fn parse_x_forwarded_for(value: &str, trusted: &[IpNet]) -> Option<IpAddr> {
    let hops: Vec<IpAddr> = value
        .split(',')
        .filter_map(|s| s.trim().parse().ok())
        .collect();

    for ip in hops.iter().rev() {
        if !trusted.iter().any(|cidr| cidr.contains(ip)) {
            return Some(*ip);
        }
    }

    // Every hop is a trusted proxy (e.g. internal health checks).
    hops.first().copied()
}

fn parse_x_forwarded_proto(value: &str) -> Option<ResolvedScheme> {
    let token = value.split(',').next()?.trim();
    match token.to_ascii_lowercase().as_str() {
        "https" => Some(ResolvedScheme::Https),
        "http" => Some(ResolvedScheme::Http),
        _ => None,
    }
}

/// Inserts [`RequestPeer`] into request extensions.
pub async fn request_context_middleware(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    let trusted = peer_in_trusted_proxies(&addr, &state.trusted_proxy_cidrs);

    let (client_ip, scheme) = if trusted {
        let ip = request
            .headers()
            .get("x-forwarded-for")
            .and_then(|h| h.to_str().ok())
            .and_then(|v| parse_x_forwarded_for(v, &state.trusted_proxy_cidrs))
            .unwrap_or_else(|| addr.ip());

        // Fail closed: behind a trusted proxy we require an explicit forwarded scheme.
        let scheme = request
            .headers()
            .get("x-forwarded-proto")
            .and_then(|h| h.to_str().ok())
            .and_then(parse_x_forwarded_proto)
            .unwrap_or(ResolvedScheme::Http);

        (ip, scheme)
    } else {
        (addr.ip(), ResolvedScheme::Unknown)
    };

    request.extensions_mut().insert(RequestPeer {
        client_ip,
        scheme,
    });

    next.run(request).await
}

#[cfg(test)]
mod tests {
    use super::{parse_x_forwarded_for, parse_x_forwarded_proto, peer_in_trusted_proxies, ResolvedScheme};
    use ipnet::IpNet;
    use std::net::{IpAddr, SocketAddr};

    #[test]
    fn forwarded_for_rightmost_untrusted_hop() {
        let trusted: Vec<IpNet> = vec!["10.0.0.0/8".parse().expect("cidr")];
        // 10.0.0.1 is our proxy; 203.0.113.1 is the real client even though a
        // spoofed entry (198.51.100.7) was prepended by the client.
        let ip = parse_x_forwarded_for("198.51.100.7, 203.0.113.1, 10.0.0.1", &trusted)
            .expect("parse");
        assert_eq!(ip, IpAddr::from([203, 0, 113, 1]));
    }

    #[test]
    fn forwarded_for_all_trusted_falls_back_to_first() {
        let trusted: Vec<IpNet> = vec!["10.0.0.0/8".parse().expect("cidr")];
        let ip = parse_x_forwarded_for("10.0.0.2, 10.0.0.1", &trusted).expect("parse");
        assert_eq!(ip, IpAddr::from([10, 0, 0, 2]));
    }

    #[test]
    fn forwarded_proto_https() {
        assert_eq!(
            parse_x_forwarded_proto("https, http"),
            Some(ResolvedScheme::Https)
        );
    }

    #[test]
    fn trusted_proxy_cidr_contains_loopback() {
        let cidr: IpNet = "127.0.0.1/32".parse().expect("cidr");
        let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
        assert!(peer_in_trusted_proxies(&addr, std::slice::from_ref(&cidr)));
    }

    #[test]
    fn untrusted_peer_not_in_cidr() {
        let cidr: IpNet = "10.0.0.0/8".parse().expect("cidr");
        let addr = SocketAddr::from(([192, 168, 1, 1], 8080));
        assert!(!peer_in_trusted_proxies(&addr, std::slice::from_ref(&cidr)));
    }
}
