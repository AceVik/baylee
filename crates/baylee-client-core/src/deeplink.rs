//! What a `baylee:` link may ask of the client, read strictly
//! (`docs/client.md` §"Links from the website").
//!
//! A link is the website's, or any page's: whoever wrote it, the client
//! reads it as a stranger's words. So a link carries no secret and no text
//! for the client to show, and it asks for one of three things only. Anything
//! that is not exactly one of them is no link at all, and the caller drops it
//! without a word to the player.
//!
//! | Link | Asks |
//! | --- | --- |
//! | `baylee://open` | start the client, or bring it forward |
//! | `baylee://signin` | as `open`, then Baylee Central's sign-in |
//! | `baylee://gateway/add?origin=<origin>` | to propose a gateway |
//!
//! The origin is percent-encoded, an `https` address of a DNS name with an
//! optional port and nothing else. [`read`] answers it spelled as a saved
//! address is (`https://host[:port]`), which is what the client shows when it
//! asks, and what it saves when the player says yes.

use std::collections::VecDeque;
use std::time::Duration;

/// The scheme every link starts with.
pub const SCHEME: &str = "baylee";

/// The longest link read, in bytes. A website's link is a few dozen.
pub const MAX_LEN: usize = 2048;

/// How many links a minute are read; the rest are dropped ([`Pace`]).
pub const PER_MINUTE: usize = 10;

/// What a link asks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Link {
    /// Start the client, or bring it forward.
    Open,
    /// As [`Link::Open`], then Baylee Central's sign-in where nobody is
    /// signed in.
    SignIn,
    /// Add this gateway, once the player has seen it and said yes: an
    /// origin as [`read`] spells it.
    AddGateway(String),
}

impl Link {
    /// What the log may say about a link: its kind, never its text.
    #[must_use]
    pub fn kind(&self) -> &'static str {
        match self {
            Link::Open => "an open link",
            Link::SignIn => "a sign-in link",
            Link::AddGateway(_) => "a gateway link",
        }
    }
}

/// Which origins a gateway link may name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Origins {
    /// `https` and a DNS name only: every build a player runs.
    Public,
    /// Also a numeric loopback `http` origin, for a `dev-control` build's
    /// tests against a local gateway.
    AlsoLoopback,
}

/// Reads a link, or answers `None` for anything that is not exactly one.
#[must_use]
pub fn read(text: &str, origins: Origins) -> Option<Link> {
    if text.len() > MAX_LEN || !text.is_ascii() {
        return None;
    }
    let url = url::Url::parse(text).ok()?;
    if url.scheme() != SCHEME
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some()
        || url.fragment().is_some()
    {
        return None;
    }
    match url.host_str()? {
        "open" if bare(&url) => Some(Link::Open),
        "signin" if bare(&url) => Some(Link::SignIn),
        "gateway" if url.path() == "/add" => {
            let mut pairs = url.query_pairs();
            let (key, value) = pairs.next()?;
            if key != "origin" || pairs.next().is_some() {
                return None;
            }
            gateway_origin(&value, origins).map(Link::AddGateway)
        }
        _ => None,
    }
}

/// No path but the root, no query.
fn bare(url: &url::Url) -> bool {
    matches!(url.path(), "" | "/") && url.query().is_none()
}

/// A gateway link's origin, spelled as a saved address is, or `None`.
#[must_use]
pub fn gateway_origin(text: &str, origins: Origins) -> Option<String> {
    let url = url::Url::parse(text).ok()?;
    if !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || url.path() != "/"
    {
        return None;
    }
    let allowed = match (url.scheme(), url.host()?) {
        // A DNS name of at least two labels: no IP literal, and no single
        // name a local network resolves (`localhost`, a printer's).
        ("https", url::Host::Domain(name)) => name.contains('.') && !name.ends_with('.'),
        ("http", url::Host::Ipv4(ip)) => origins == Origins::AlsoLoopback && ip.is_loopback(),
        ("http", url::Host::Ipv6(ip)) => origins == Origins::AlsoLoopback && ip.is_loopback(),
        _ => false,
    };
    allowed.then(|| url.as_str().trim_end_matches('/').to_string())
}

/// How many links have been read lately: at most [`PER_MINUTE`] in any
/// minute, so a page firing links in a loop is read ten times and then not
/// at all until it stops.
#[derive(Clone, Debug, Default)]
pub struct Pace {
    read: VecDeque<Duration>,
}

impl Pace {
    /// Whether a link arriving at `now` (any clock that only moves forward)
    /// is read; one that is counts.
    pub fn admit(&mut self, now: Duration) -> bool {
        while self
            .read
            .front()
            .is_some_and(|at| now.saturating_sub(*at) >= Duration::from_secs(60))
        {
            self.read.pop_front();
        }
        if self.read.len() >= PER_MINUTE {
            return false;
        }
        self.read.push_back(now);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn add(origin: &str) -> Link {
        Link::AddGateway(origin.into())
    }

    #[test]
    fn the_three_links_are_read() {
        assert_eq!(read("baylee://open", Origins::Public), Some(Link::Open));
        assert_eq!(read("baylee://open/", Origins::Public), Some(Link::Open));
        assert_eq!(read("baylee://signin", Origins::Public), Some(Link::SignIn));
        // A scheme is read in any case (RFC 3986 §3.1); the rest is exact.
        assert_eq!(read("Baylee://open", Origins::Public), Some(Link::Open));
        assert_eq!(
            read(
                "baylee://gateway/add?origin=https%3A%2F%2Fgateway.example.org",
                Origins::Public
            ),
            Some(add("https://gateway.example.org"))
        );
        // The website's encodeURIComponent, a port, a trailing slash.
        assert_eq!(
            read(
                "baylee://gateway/add?origin=https%3A%2F%2Fplay.example.org%3A8443%2F",
                Origins::Public
            ),
            Some(add("https://play.example.org:8443"))
        );
        // An origin left unencoded still reads as the same origin.
        assert_eq!(
            read(
                "baylee://gateway/add?origin=https://gateway.example.org",
                Origins::Public
            ),
            Some(add("https://gateway.example.org"))
        );
    }

    #[test]
    fn a_saved_address_is_spelled_the_way_the_typed_one_is() {
        // Upper case and the default port fall away, an IDN reads as its
        // ASCII form: what the player is asked about is what is saved.
        assert_eq!(
            gateway_origin("HTTPS://Gateway.Example.ORG:443/", Origins::Public).as_deref(),
            Some("https://gateway.example.org")
        );
        assert_eq!(
            gateway_origin("https://bücher.example", Origins::Public).as_deref(),
            Some("https://xn--bcher-kva.example")
        );
    }

    #[test]
    fn anything_else_is_no_link() {
        for text in [
            "",
            "baylee:",
            "baylee://",
            "baylee://OPEN",
            "https://open",
            "baylee://open?x=1",
            "baylee://open/more",
            "baylee://open#top",
            "baylee://signin?username=someone",
            "baylee://user@open",
            "baylee://open:1",
            "baylee://gateway/add",
            "baylee://gateway/add/",
            "baylee://gateway/remove?origin=https%3A%2F%2Fa.example",
            "baylee://gateway/add?origin=",
            "baylee://gateway/add?url=https%3A%2F%2Fa.example",
            "baylee://gateway/add?origin=https%3A%2F%2Fa.example&name=Official",
            "baylee://gateway/add?origin=https%3A%2F%2Fa.example&origin=https%3A%2F%2Fb.example",
            "baylee://gateway/add?name=x&origin=https%3A%2F%2Fa.example",
            "baylee://gateway/add?origin=https%3A%2F%2Fa.example#x",
            "baylee://gateway/add?origin=https%3A%2F%2Fa.example\u{e9}",
        ] {
            assert_eq!(read(text, Origins::AlsoLoopback), None, "{text}");
        }
        let long = format!(
            "baylee://gateway/add?origin=https%3A%2F%2F{}.example",
            "a".repeat(MAX_LEN)
        );
        assert_eq!(read(&long, Origins::Public), None);
    }

    #[test]
    fn a_gateway_link_names_a_public_https_name_and_nothing_more() {
        for origin in [
            "http://gateway.example.org",
            "https://gateway.example.org/base",
            "https://gateway.example.org/?q=1",
            "https://gateway.example.org/#x",
            "https://user:pass@gateway.example.org",
            "https://user@gateway.example.org",
            "https://203.0.113.7",
            "https://[2001:db8::1]",
            "https://localhost",
            "https://printer",
            "https://gateway.example.org.",
            "ftp://gateway.example.org",
            "javascript:alert(1)",
            "file:///etc/passwd",
            "http://127.0.0.1:28766",
        ] {
            assert_eq!(gateway_origin(origin, Origins::Public), None, "{origin}");
        }
    }

    #[test]
    fn a_dev_build_also_takes_a_numeric_loopback_gateway() {
        assert_eq!(
            gateway_origin("http://127.0.0.1:28766", Origins::AlsoLoopback).as_deref(),
            Some("http://127.0.0.1:28766")
        );
        assert_eq!(
            gateway_origin("http://[::1]:28766/", Origins::AlsoLoopback).as_deref(),
            Some("http://[::1]:28766")
        );
        for origin in ["http://localhost:28766", "http://192.168.1.5:28766"] {
            assert_eq!(
                gateway_origin(origin, Origins::AlsoLoopback),
                None,
                "{origin}"
            );
        }
    }

    #[test]
    fn the_log_names_a_link_by_its_kind_only() {
        let link = read(
            "baylee://gateway/add?origin=https%3A%2F%2Fsecret-name.example",
            Origins::Public,
        )
        .expect("a link");
        assert!(!link.kind().contains("secret-name"));
    }

    #[test]
    fn ten_links_a_minute_are_read_and_the_rest_dropped() {
        let mut pace = Pace::default();
        let second = |s: u64| Duration::from_secs(s);
        for n in 0..10 {
            assert!(pace.admit(second(n)), "link {n}");
        }
        assert!(!pace.admit(second(10)));
        assert!(!pace.admit(second(59)));
        // The first one is a minute old: one more is read.
        assert!(pace.admit(second(60)));
        assert!(!pace.admit(second(60)));
    }

    /// Every byte string a link could be is answered without a panic, and
    /// whatever is read as a gateway names an origin that reads again as
    /// itself (a deterministic sweep standing in for a fuzzer).
    #[test]
    fn no_text_upsets_the_reader() {
        let pieces = [
            "baylee://",
            "gateway",
            "/add",
            "?origin=",
            "https%3A%2F%2F",
            "https://",
            "a.example",
            "%2F",
            "%00",
            "%",
            "&",
            "=",
            "#",
            "@",
            ":",
            "[",
            "]",
            "..",
            "%E2%80%AE",
            "open",
            "signin",
            "\\",
            " ",
            "+",
        ];
        let mut seed = 0x9e37_79b9_u32;
        let mut next = || {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            usize::try_from(seed).expect("a u32 fits")
        };
        for _ in 0..20_000 {
            let mut text = String::new();
            for _ in 0..next() % 7 {
                text.push_str(pieces[next() % pieces.len()]);
            }
            if let Some(Link::AddGateway(origin)) = read(&text, Origins::AlsoLoopback) {
                assert_eq!(
                    gateway_origin(&origin, Origins::AlsoLoopback).as_deref(),
                    Some(origin.as_str()),
                    "{text}"
                );
            }
        }
    }
}
