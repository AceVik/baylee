//! What a `baylee:` link may ask of the client, read strictly
//! (`docs/client.md` §"Links from the website").
//!
//! A link is the website's, or any page's: whoever wrote it, the client
//! reads it as a stranger's words. So a link carries no text for the client
//! to show, and it asks for one of four things only. Anything that is not
//! exactly one of them is no link at all, and the caller drops it without a
//! word to the player.
//!
//! | Link | Asks |
//! | --- | --- |
//! | `baylee://open` | start the client, or bring it forward |
//! | `baylee://signin` | as `open`, then Baylee Central's sign-in |
//! | `baylee://gateway/add?origin=<origin>` | to propose a gateway |
//! | `baylee://pair?h=<handle>&s=<secret>` | to sign in from baylee.app |
//!
//! The origin is percent-encoded, an `https` address of a DNS name with an
//! optional port and nothing else. [`read`] answers it spelled as a saved
//! address is (`https://host[:port]`), which is what the client shows when it
//! asks, and what it saves when the player says yes.
//!
//! The pairing link is the one that carries a secret (`docs/proposals/
//! deep-links.md` §"The link `baylee://pair`"): the page's one-time `S`, good
//! for the two minutes a pairing may be claimed in. Both values are 32 bytes
//! written as 43 characters of URL-safe base64 without padding, in this
//! order and spelled exactly so: no padding, no percent-encoding, no other
//! parameter. [`PairLink`] wipes them when dropped and never prints them.

use std::collections::VecDeque;
use std::time::Duration;
use zeroize::Zeroizing;

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
    /// Sign in from baylee.app: claim the pairing the page started, with the
    /// page's secret, where nobody is signed in.
    Pair(PairLink),
}

impl Link {
    /// What the log may say about a link: its kind, never its text.
    #[must_use]
    pub fn kind(&self) -> &'static str {
        match self {
            Link::Open => "an open link",
            Link::SignIn => "a sign-in link",
            Link::AddGateway(_) => "a gateway link",
            Link::Pair(_) => "a pairing link",
        }
    }
}

/// How many characters a pairing link's handle and secret each are: 32
/// bytes in URL-safe base64 without padding.
pub const PAIR_VALUE_CHARS: usize = 43;

/// A pairing link's two values: the pairing's handle, which Central keeps
/// only as a digest, and the page's one-time secret, which Central never
/// sees. Together they claim the pairing, so both are wiped when dropped and
/// neither is ever printed.
#[derive(Clone, PartialEq, Eq)]
pub struct PairLink {
    handle: Zeroizing<[u8; 32]>,
    secret: Zeroizing<[u8; 32]>,
}

impl PairLink {
    /// A pairing link's values, as the page made them.
    #[must_use]
    pub fn new(handle: [u8; 32], secret: [u8; 32]) -> Self {
        Self {
            handle: Zeroizing::new(handle),
            secret: Zeroizing::new(secret),
        }
    }

    /// The pairing's handle.
    #[must_use]
    pub fn handle(&self) -> &[u8; 32] {
        &self.handle
    }

    /// The page's one-time secret.
    #[must_use]
    pub fn secret(&self) -> &[u8; 32] {
        &self.secret
    }
}

impl std::fmt::Debug for PairLink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PairLink([REDACTED])")
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
        "pair" if matches!(url.path(), "" | "/") => pair(url.query()?).map(Link::Pair),
        _ => None,
    }
}

/// No path but the root, no query.
fn bare(url: &url::Url) -> bool {
    matches!(url.path(), "" | "/") && url.query().is_none()
}

/// A pairing link's query, read as written and never percent-decoded:
/// exactly `h=<handle>&s=<secret>`, each value 43 characters that decode to
/// 32 bytes as their encoder would have written them.
fn pair(query: &str) -> Option<PairLink> {
    let (handle, secret) = query.strip_prefix("h=")?.split_once("&s=")?;
    Some(PairLink {
        handle: pair_value(handle)?,
        secret: pair_value(secret)?,
    })
}

/// One of a pairing link's values: 43 characters of URL-safe base64 without
/// padding and with no bit set past the 32nd byte. The decoded bytes are
/// wiped once copied.
fn pair_value(text: &str) -> Option<Zeroizing<[u8; 32]>> {
    if text.len() != PAIR_VALUE_CHARS {
        return None;
    }
    let bytes = Zeroizing::new(baylee_seal::b64::url::decode(text).ok()?);
    if bytes.len() != 32 {
        return None;
    }
    let mut value = Zeroizing::new([0; 32]);
    value.copy_from_slice(&bytes);
    Some(value)
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
        let link = read(&pair_text(&HANDLE, &SECRET), Origins::Public).expect("a link");
        assert_eq!(link.kind(), "a pairing link");
    }

    const HANDLE: [u8; 32] = [0xa5; 32];
    /// Two `0xff` at a group's end write `_`, which only URL-safe base64 has.
    const SECRET: [u8; 32] = [
        1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
        26, 27, 28, 0xff, 0xff, 31, 250,
    ];

    /// The link as the page writes it.
    fn pair_text(handle: &[u8; 32], secret: &[u8; 32]) -> String {
        let (h, s) = (b64(handle), b64(secret));
        format!("baylee://pair?h={h}&s={s}")
    }

    fn b64(bytes: &[u8]) -> String {
        baylee_seal::b64::url::encode(bytes)
    }

    #[test]
    fn a_pairing_link_is_read_exactly_as_the_page_writes_it() {
        let text = pair_text(&HANDLE, &SECRET);
        assert_eq!(
            text.len(),
            "baylee://pair?h=&s=".len() + 2 * PAIR_VALUE_CHARS
        );
        let Some(Link::Pair(link)) = read(&text, Origins::Public) else {
            panic!("a pairing link");
        };
        assert_eq!(link.handle(), &HANDLE);
        assert_eq!(link.secret(), &SECRET);
        assert_eq!(link, PairLink::new(HANDLE, SECRET));
        // As the other links: a root path and the scheme in any case.
        let (h, s) = (b64(&HANDLE), b64(&SECRET));
        for text in [
            format!("baylee://pair/?h={h}&s={s}"),
            format!("Baylee://pair?h={h}&s={s}"),
        ] {
            assert_eq!(
                read(&text, Origins::Public),
                Some(Link::Pair(PairLink::new(HANDLE, SECRET))),
                "{text}"
            );
        }
    }

    #[test]
    fn a_pairing_link_takes_its_two_values_spelled_one_way_only() {
        let (h, s) = (b64(&HANDLE), b64(&SECRET));
        // The last character of a 32-byte value carries two bits past the
        // last byte, which the encoder leaves clear.
        let mut loose = s.clone();
        loose.pop();
        loose.push('B');
        let percent = format!("%{:02X}{}", s.as_bytes()[0], &s[1..]);
        let standard = s.replace('-', "+").replace('_', "/");
        for text in [
            format!("baylee://pair?s={s}&h={h}"),
            format!("baylee://pair?h={h}"),
            format!("baylee://pair?s={s}"),
            format!("baylee://pair?h={h}&s={s}&x=1"),
            format!("baylee://pair?h={h}&s={s}&s={s}"),
            format!("baylee://pair?h={h}&h={h}&s={s}"),
            format!("baylee://pair?x=1&h={h}&s={s}"),
            format!("baylee://pair?H={h}&S={s}"),
            format!("baylee://pair?h={h}&s={s}="),
            format!("baylee://pair?h={h}=&s={s}"),
            format!("baylee://pair?h={h}&s={}", &s[1..]),
            format!("baylee://pair?h={}&s={s}", &h[1..]),
            format!("baylee://pair?h={h}&s={s}A"),
            format!("baylee://pair?h={h}&s={loose}"),
            format!("baylee://pair?h={h}&s={percent}"),
            format!("baylee://pair?h={h}&s={standard}"),
            format!("baylee://pair?h=&s={s}"),
            format!("baylee://pair?h={h}&s="),
            format!("baylee://pair?h={h}&s={s}#x"),
            format!("baylee://pair/x?h={h}&s={s}"),
            format!("baylee://PAIR?h={h}&s={s}"),
            format!("baylee://user@pair?h={h}&s={s}"),
            format!("baylee://pair:1?h={h}&s={s}"),
            format!("baylee://pair?h={h}%26s={s}"),
            format!("baylee://pair?h= {h}&s={s}"),
            format!("baylee://open?h={h}&s={s}"),
            "baylee://pair".to_string(),
            "baylee://pair?".to_string(),
        ] {
            assert_eq!(read(&text, Origins::AlsoLoopback), None, "{text}");
        }
        assert_ne!(
            standard, s,
            "the sample has a character only URL-safe base64 writes"
        );
    }

    #[test]
    fn a_pairing_link_never_prints_its_values() {
        let link = read(&pair_text(&HANDLE, &SECRET), Origins::Public).expect("a link");
        let printed = format!("{link:?}");
        assert_eq!(printed, "Pair(PairLink([REDACTED]))");
        for value in [b64(&HANDLE), b64(&SECRET)] {
            assert!(!printed.contains(&value));
        }
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
    /// itself, and whatever is read as a pairing is the one spelling of its
    /// values (a deterministic sweep standing in for a fuzzer).
    #[test]
    fn no_text_upsets_the_reader() {
        let (h, s) = (b64(&HANDLE), b64(&SECRET));
        let pieces = [
            "pair",
            "?h=",
            "&s=",
            h.as_str(),
            s.as_str(),
            &s[..42],
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
            match read(&text, Origins::AlsoLoopback) {
                Some(Link::AddGateway(origin)) => assert_eq!(
                    gateway_origin(&origin, Origins::AlsoLoopback).as_deref(),
                    Some(origin.as_str()),
                    "{text}"
                ),
                Some(Link::Pair(link)) => {
                    let written = pair_text(link.handle(), link.secret());
                    assert!(text.ends_with(&written["baylee://".len()..]), "{text}");
                }
                _ => {}
            }
        }
    }
}
