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
    /// Open a public challenge detail; never begin a game or reveal a hint.
    Challenge([u8; 16]),
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
            Link::Challenge(_) => "a challenge link",
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

/// Reads a link, or answers `None` for anything that is not exactly one:
/// printable ASCII only, so no space, tab or line break that a URL parser
/// would quietly strip or trim.
#[must_use]
pub fn read(text: &str, origins: Origins) -> Option<Link> {
    if text.len() > MAX_LEN || !text.bytes().all(|byte| byte.is_ascii_graphic()) {
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
        "challenge" if url.query().is_none() => {
            let id = url.path().strip_prefix('/')?;
            if id.len() != 32
                || !id
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            {
                return None;
            }
            let id = uuid::Uuid::parse_str(id).ok()?;
            (id.get_version_num() == 7 && id.get_variant() == uuid::Variant::RFC4122)
                .then(|| Link::Challenge(*id.as_bytes()))
        }
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
mod tests;
