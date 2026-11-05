//! Same-origin admission: the exact host names a browser may use, and a
//! refusal a person on a phone can read.
//!
//! A request must name one of the accepted authorities in its `Host` header,
//! which stops DNS rebinding, and a browser's `Origin`, when sent, must be that
//! same authority over `http`, which stops cross-origin reads.

use axum::{
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use std::{
    collections::BTreeSet,
    fmt::Write as _,
    net::{Ipv4Addr, Ipv6Addr, SocketAddr},
    sync::{Arc, Mutex, PoisonError},
};

/// Distinct rejected host names logged per process, so a phone's mistake is
/// visible in the journal without letting a scanner fill it.
const LOGGED_REJECTIONS: usize = 16;

#[derive(Clone)]
pub(crate) struct Admission {
    /// Lowercase `host:port` values: the bind address first, then the
    /// operator's `--allow-host` names.
    authorities: Arc<[String]>,
    logged: Arc<Mutex<BTreeSet<String>>>,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Rejection {
    /// The `Host` header is missing or names no accepted authority.
    UnknownHost(Option<String>),
    /// A browser reported another site as the request's origin.
    CrossOrigin,
}

impl Admission {
    pub fn new(bind: SocketAddr, allowed: &[String]) -> Self {
        let mut authorities = vec![bind.to_string()];
        for name in allowed {
            if !authorities.contains(name) {
                authorities.push(name.clone());
            }
        }
        Self {
            authorities: authorities.into(),
            logged: Arc::default(),
        }
    }

    /// Every URL this service answers to, in configuration order.
    pub fn urls(&self) -> Vec<String> {
        self.authorities
            .iter()
            .map(|authority| format!("http://{authority}"))
            .collect()
    }

    pub fn check(&self, headers: &HeaderMap) -> Result<(), Rejection> {
        let host = headers
            .get(header::HOST)
            .and_then(|v| v.to_str().ok())
            .map(str::to_ascii_lowercase);
        let Some(host) = host.filter(|h| self.authorities.contains(h)) else {
            let shown = headers.get(header::HOST).map(printable);
            return Err(Rejection::UnknownHost(shown));
        };
        if headers
            .get("sec-fetch-site")
            .is_some_and(|v| v == "cross-site")
        {
            return Err(Rejection::CrossOrigin);
        }
        let same_origin = headers.get(header::ORIGIN).is_none_or(|origin| {
            origin
                .to_str()
                .is_ok_and(|o| o.eq_ignore_ascii_case(&format!("http://{host}")))
        });
        if same_origin {
            Ok(())
        } else {
            Err(Rejection::CrossOrigin)
        }
    }

    /// A short plain-text 403 that says which addresses work.
    pub fn refuse(&self, rejection: &Rejection) -> Response {
        let reason = match rejection {
            Rejection::UnknownHost(host) => {
                self.log_once(host.as_deref().unwrap_or("(none)"));
                match host {
                    Some(host) => format!("Dein Browser hat die Adresse „{host}“ verwendet."),
                    None => "Dein Browser hat keine Adresse (Host) mitgeschickt.".to_owned(),
                }
            }
            Rejection::CrossOrigin => {
                "Die Anfrage kam von einer anderen Webseite; das ist nicht erlaubt.".to_owned()
            }
        };
        let mut body = format!(
            "Baylee Pulse: Zugriff abgelehnt (403).\n\n{reason}\n\n\
             Öffne genau eine dieser Adressen, mit http:// und nicht https://:\n"
        );
        for url in self.urls() {
            let _ = writeln!(body, "  {url}");
        }
        body.push_str("\nWeitere Namen kann der Betreiber mit --allow-host freigeben.\n");
        (
            StatusCode::FORBIDDEN,
            [(
                header::CONTENT_TYPE,
                HeaderValue::from_static("text/plain; charset=utf-8"),
            )],
            body,
        )
            .into_response()
    }

    fn log_once(&self, host: &str) {
        let mut logged = self.logged.lock().unwrap_or_else(PoisonError::into_inner);
        if logged.len() < LOGGED_REJECTIONS && logged.insert(host.to_owned()) {
            eprintln!(
                "Baylee Pulse: refused Host {host:?}; accepted: {}",
                self.urls().join(", ")
            );
        }
    }
}

/// The `Host` value as safe, short display text.
fn printable(value: &HeaderValue) -> String {
    value
        .as_bytes()
        .iter()
        .take(80)
        .map(|&b| {
            if b.is_ascii_graphic() {
                char::from(b)
            } else {
                '?'
            }
        })
        .collect()
}

/// Parses one `--allow-host` value: an exact `name:port`, `ipv4:port` or
/// `[ipv6]:port`. No wildcards and no default port; returned lowercase.
pub(crate) fn parse_allowed_host(value: &str) -> Result<String, String> {
    let value = value.to_ascii_lowercase();
    let (host, port) = value
        .rsplit_once(':')
        .ok_or("expected HOST:PORT, for example bigace.local:8787")?;
    if !port.bytes().all(|b| b.is_ascii_digit()) || port.parse::<u16>().map_or(true, |p| p == 0) {
        return Err(format!("invalid port {port:?}"));
    }
    let valid = if let Some(ip) = host.strip_prefix('[').and_then(|h| h.strip_suffix(']')) {
        ip.parse::<Ipv6Addr>().is_ok()
    } else {
        host.parse::<Ipv4Addr>().is_ok() || is_dns_name(host)
    };
    if valid {
        Ok(value)
    } else {
        Err(format!(
            "{host:?} is not an exact host name or IP address (wildcards are not accepted)"
        ))
    }
}

fn is_dns_name(host: &str) -> bool {
    !host.is_empty()
        && host.len() <= 253
        && host.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn admission() -> Admission {
        Admission::new(
            "192.168.0.103:8787".parse().unwrap(),
            &["bigace.local:8787".into()],
        )
    }

    fn headers(pairs: &[(&'static str, &'static str)]) -> HeaderMap {
        let mut h = HeaderMap::new();
        for (name, value) in pairs {
            h.insert(*name, HeaderValue::from_static(value));
        }
        h
    }

    #[test]
    fn prevents_cross_origin_reads_and_dns_rebinding() {
        let a = admission();
        assert_eq!(a.check(&headers(&[("host", "192.168.0.103:8787")])), Ok(()));
        assert_eq!(
            a.check(&headers(&[
                ("host", "192.168.0.103:8787"),
                ("origin", "http://attacker.invalid")
            ])),
            Err(Rejection::CrossOrigin)
        );
        assert_eq!(
            a.check(&headers(&[
                ("host", "192.168.0.103:8787"),
                ("origin", "http://192.168.0.103:8787")
            ])),
            Ok(())
        );
        assert_eq!(
            a.check(&headers(&[("host", "rebound.invalid:8787")])),
            Err(Rejection::UnknownHost(Some("rebound.invalid:8787".into())))
        );
        assert_eq!(
            a.check(&HeaderMap::new()),
            Err(Rejection::UnknownHost(None))
        );
        assert_eq!(
            a.check(&headers(&[
                ("host", "192.168.0.103:8787"),
                ("sec-fetch-site", "cross-site")
            ])),
            Err(Rejection::CrossOrigin)
        );
    }

    #[test]
    fn accepts_operator_names_exactly_and_only_as_their_own_origin() {
        let a = admission();
        assert_eq!(a.check(&headers(&[("host", "BigAce.local:8787")])), Ok(()));
        assert_eq!(
            a.check(&headers(&[
                ("host", "bigace.local:8787"),
                ("origin", "http://bigace.local:8787")
            ])),
            Ok(())
        );
        // Both names are accepted, but a page from one may not read the other.
        assert_eq!(
            a.check(&headers(&[
                ("host", "bigace.local:8787"),
                ("origin", "http://192.168.0.103:8787")
            ])),
            Err(Rejection::CrossOrigin)
        );
        assert!(a.check(&headers(&[("host", "bigace.local:8788")])).is_err());
        assert!(a.check(&headers(&[("host", "bigace.local")])).is_err());
        assert!(
            a.check(&headers(&[("host", "x.bigace.local:8787")]))
                .is_err()
        );
        assert_eq!(
            a.urls(),
            ["http://192.168.0.103:8787", "http://bigace.local:8787"]
        );
    }

    #[test]
    fn allow_host_takes_exact_names_with_a_port() {
        assert_eq!(
            parse_allowed_host("BigAce.Fritz.Box:8787"),
            Ok("bigace.fritz.box:8787".into())
        );
        assert_eq!(parse_allowed_host("bigace:8787"), Ok("bigace:8787".into()));
        assert_eq!(parse_allowed_host("10.0.0.2:80"), Ok("10.0.0.2:80".into()));
        assert_eq!(
            parse_allowed_host("[fe80::1]:8787"),
            Ok("[fe80::1]:8787".into())
        );
        for bad in [
            "bigace",
            "*.local:8787",
            "bigace.local:0",
            "bigace.local:99999",
            "bigace.local:",
            ":8787",
            "-bad.local:8787",
            "a..b:8787",
            "[nonsense]:8787",
            "fe80::1:8787",
            "http://bigace:8787",
            "bigace:+8787",
        ] {
            assert!(parse_allowed_host(bad).is_err(), "{bad} was accepted");
        }
    }

    #[tokio::test]
    async fn refusal_names_the_working_addresses_in_plain_text() {
        let a = admission();
        let response = a.refuse(&Rejection::UnknownHost(Some(
            "bigace.fritz.box:8787".into(),
        )));
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_eq!(
            response.headers()[header::CONTENT_TYPE],
            "text/plain; charset=utf-8"
        );
        let body = axum::body::to_bytes(response.into_body(), 4096)
            .await
            .unwrap();
        let body = std::str::from_utf8(&body).unwrap();
        assert!(body.contains("„bigace.fritz.box:8787“"), "{body}");
        assert!(body.contains("  http://192.168.0.103:8787\n"), "{body}");
        assert!(body.contains("  http://bigace.local:8787\n"), "{body}");
        assert!(body.contains("nicht https://"), "{body}");
        assert_eq!(printable(&HeaderValue::from_static("a\tb")), "a?b");
    }
}
