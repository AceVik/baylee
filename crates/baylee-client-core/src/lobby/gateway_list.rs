//! The saved gateway list and the one entry in it nobody can remove.
//!
//! The live gateway, "Baylee Sanctuary", is where a player who has never
//! typed an address plays. It is always in the list, and forgetting it is
//! refused here rather than only hidden in the interface: a button taken
//! away is one path, a rule in the list is every path.

/// The live gateway's address, spelled as a saved address is: scheme and
/// host, no trailing slash. The gateway serves at `https://gateway.baylee.app`
/// too since 30.10.2026; the client follows once Baylee Central takes tickets
/// for that origin (#402: this becomes the new address, [`FORMER`] the old).
pub const PINNED: &str = "https://baylee.acevik.de";

/// The live gateway's former addresses, which [`moved`] reads as [`PINNED`].
/// None yet: the client has not followed the gateway to its new address
/// (#402), so every function here leaves the live gateway's address as it is.
pub const FORMER: &[&str] = &[];

/// What the live gateway calls itself (its `BAYLEE_GATEWAY_NAME`).
pub const PINNED_NAME: &str = "Baylee Sanctuary";

/// Whether an address is the pinned one.
#[must_use]
pub fn is_pinned(url: &str) -> bool {
    url == PINNED
}

/// An address as it is spelled today: one of the live gateway's
/// [`FORMER`] addresses, with or without a trailing slash, is [`PINNED`];
/// every other address is itself.
#[must_use]
pub fn moved(url: &str) -> &str {
    if FORMER.contains(&url.trim_end_matches('/')) {
        PINNED
    } else {
        url
    }
}

/// Whether two addresses name one gateway: the same address, a trailing
/// slash aside, or the live gateway by its new and a [`FORMER`] address.
/// The one comparison of gateway addresses; nothing compares them itself.
#[must_use]
pub fn same(a: &str, b: &str) -> bool {
    moved(a.trim_end_matches('/')) == moved(b.trim_end_matches('/'))
}

/// The address a gateway's keys are bound to: for the live gateway its
/// first address, [`KEYED`]; for every other gateway its own address.
#[must_use]
pub fn keyed(url: &str) -> &str {
    if moved(url) == PINNED { KEYED } else { url }
}

/// The address the live gateway's keys are bound to, before and after its
/// move: the vault's scope (its folder name and its associated data), the
/// chat folder, the keychain slot and the account's chat records name the
/// live gateway by it ([`keyed`]), so a key made on either side of the move
/// is found on the other, and an installed client on another device finds
/// the chat records a new one writes. It never changes, whatever [`FORMER`]
/// becomes.
pub const KEYED: &str = "https://baylee.acevik.de";

/// A saved list with every former address of the live gateway read as
/// [`PINNED`] ([`moved`]) and each address once, where it first stood (a
/// repeated address of any gateway is dropped). Running it again changes
/// nothing.
#[must_use]
pub fn renamed(list: Vec<String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::with_capacity(list.len());
    for url in list {
        let url = if moved(&url) == PINNED {
            PINNED.to_string()
        } else {
            url
        };
        if !out.contains(&url) {
            out.push(url);
        }
    }
    out
}

/// The list as the account's synced document keeps it: the live gateway
/// under [`KEYED`], the address every client installed before the move pins,
/// so that client finds its own pinned row there and no second one it does
/// not know. A client reads it back with [`renamed`]. Running it again
/// changes nothing.
#[must_use]
pub fn filed(list: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::with_capacity(list.len());
    for url in list {
        let url = keyed(url).to_string();
        if !out.contains(&url) {
            out.push(url);
        }
    }
    out
}

/// A map by address with an entry under a former address of the live
/// gateway moved to [`PINNED`]. Where [`PINNED`] has an entry already, that
/// one stays and so does the former one: nothing kept under either address
/// is overwritten or lost. Running it again changes nothing.
pub fn rekeyed<V>(map: &mut std::collections::BTreeMap<String, V>) {
    let former: Vec<String> = map
        .keys()
        .filter(|key| key.as_str() != PINNED && moved(key) == PINNED)
        .cloned()
        .collect();
    for key in former {
        if !map.contains_key(PINNED)
            && let Some(value) = map.remove(&key)
        {
            map.insert(PINNED.to_string(), value);
        }
    }
}

/// Normalizes a saved gateway list: keeps its order and reads a former
/// address of the live gateway as [`PINNED`] ([`renamed`]). Does not add
/// any gateway that was not already saved.
#[must_use]
pub fn with_pinned(list: Vec<String>) -> Vec<String> {
    renamed(list)
}

/// Removes an address from the list. Answers whether anything was removed.
pub fn forget(list: &mut Vec<String>, url: &str) -> bool {
    let before = list.len();
    list.retain(|saved| saved != url);
    list.len() != before
}

/// The row the keyboard's cursor starts on: the pinned gateway in a release
/// build, which is where a player is sent; none in a debug build, whose
/// developer is sent to their own gateway by the address field instead.
#[must_use]
pub fn preselected(list: &[String], release: bool) -> Option<usize> {
    release.then(|| list.iter().position(|url| is_pinned(url)))?
}

/// The gateway that served the page, in a browser (#327).
///
/// `page_origin` is `window.location.origin` (`None` natively, where there is
/// no page), and `configured` the address the client was started with. A
/// browser client told no other address (`?gateway=…`, or one a visit
/// remembered) is configured with its page's origin, and that is what serving
/// the build under the gateway's own origin (`/play/`) means: the gateway that
/// served the page is the one to play on. So the answer is the origin exactly
/// when the two agree, and only for an `http(s)` origin: an opaque one
/// (`null`, a `file:` page) names no host.
#[must_use]
pub fn page_gateway(page_origin: Option<&str>, configured: &str) -> Option<String> {
    let origin = page_origin?.trim().trim_end_matches('/');
    let served = origin
        .strip_prefix("https://")
        .or_else(|| origin.strip_prefix("http://"))
        .is_some_and(|host| !host.is_empty() && !host.contains('/'));
    (served && origin == configured.trim().trim_end_matches('/')).then(|| origin.to_string())
}

/// The list a page served by a gateway opens with: that gateway joins it at
/// the front if it is missing, after the pinned one's rule has run. Without a
/// page gateway the list is unchanged.
#[must_use]
pub fn with_page(mut list: Vec<String>, page: Option<&str>) -> Vec<String> {
    if let Some(page) = page
        && !list.iter().any(|url| url == page)
    {
        list.insert(0, page.to_string());
    }
    list
}

/// The row the keyboard's cursor starts on, the page's gateway first: a
/// browser served by a gateway starts on that gateway, in any build;
/// everywhere else [`preselected`] decides.
#[must_use]
pub fn starting_row(list: &[String], release: bool, page: Option<&str>) -> Option<usize> {
    page.and_then(|page| list.iter().position(|url| url == page))
        .or_else(|| preselected(list, release))
}

/// Which availability section of the gateway list an address belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GatewayGroup {
    /// Online and version-compatible (or still being probed).
    Compatible,
    /// Online, but speaking another protocol or view version (or pre-`/info`).
    Incompatible,
    /// Offline or not answering as a gateway.
    Offline,
}

impl GatewayGroup {
    /// The three groups in display order.
    pub const ALL: [Self; 3] = [Self::Compatible, Self::Incompatible, Self::Offline];

    /// Index into a 3-element array (`0..3`).
    #[must_use]
    pub const fn index(self) -> usize {
        match self {
            Self::Compatible => 0,
            Self::Incompatible => 1,
            Self::Offline => 2,
        }
    }

    /// Whether the section starts folded by default: the compatible gateways
    /// stay open; the incompatible and offline sections start collapsed.
    #[must_use]
    pub const fn collapsed_by_default(self) -> bool {
        match self {
            Self::Compatible => false,
            Self::Incompatible | Self::Offline => true,
        }
    }

    /// Classifies a gateway from its latest probe result.
    #[must_use]
    pub fn of(probe: Option<&super::gateway_info::Probe>, protocol: u32, view: u32) -> Self {
        match probe {
            Some(super::gateway_info::Probe::Silent) => Self::Offline,
            Some(p) if p.warning(protocol, view).is_some() => Self::Incompatible,
            _ => Self::Compatible,
        }
    }
}

/// Filter pill above the gateway list.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum GatewayFilter {
    /// Every group.
    #[default]
    All,
    /// Only online and version-compatible gateways.
    Compatible,
    /// Only online but version-incompatible gateways.
    Incompatible,
    /// Only offline gateways.
    Offline,
}

impl GatewayFilter {
    /// All filter pills in display order.
    pub const ALL: [Self; 4] = [
        Self::All,
        Self::Compatible,
        Self::Incompatible,
        Self::Offline,
    ];

    /// Whether a gateway in `group` passes this filter.
    #[must_use]
    pub const fn matches(self, group: GatewayGroup) -> bool {
        match self {
            Self::All => true,
            Self::Compatible => matches!(group, GatewayGroup::Compatible),
            Self::Incompatible => matches!(group, GatewayGroup::Incompatible),
            Self::Offline => matches!(group, GatewayGroup::Offline),
        }
    }
}

/// Sort order within each gateway group.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum GatewaySort {
    /// The account/device's recency order (`gateway_use::ordered`).
    #[default]
    LastUsed,
    /// Alphabetical by gateway name.
    Name,
    /// Alphabetical by URL/host.
    Address,
    /// Newest version first, then name.
    Version,
}

impl GatewaySort {
    /// Cycles to the next sort order.
    #[must_use]
    pub const fn next(self) -> Self {
        match self {
            Self::LastUsed => Self::Name,
            Self::Name => Self::Address,
            Self::Address => Self::Version,
            Self::Version => Self::LastUsed,
        }
    }
}

/// The name a gateway row shows as its primary title.
#[must_use]
pub fn display_name<'a>(url: &'a str, probe: Option<&'a super::gateway_info::Probe>) -> &'a str {
    if let Some(super::gateway_info::Probe::Known(info)) = probe
        && let Some(name) = info.name.as_deref()
    {
        return name;
    }
    if is_pinned(url) {
        return PINNED_NAME;
    }
    super::gateway_info::address_host(url).unwrap_or(url)
}

/// Whether a gateway matches the search box query.
///
/// Every whitespace-separated token in `query` must appear (ASCII
/// case-insensitively) in the gateway's name, URL, or version string.
#[must_use]
pub fn matches_query(url: &str, probe: Option<&super::gateway_info::Probe>, query: &str) -> bool {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return true;
    }
    let name = display_name(url, probe).to_ascii_lowercase();
    let addr = url.to_ascii_lowercase();
    let version = match probe {
        Some(super::gateway_info::Probe::Known(info)) => info.version.to_ascii_lowercase(),
        _ => String::new(),
    };
    trimmed.split_whitespace().all(|raw| {
        let needle = raw.to_ascii_lowercase();
        name.contains(&needle) || addr.contains(&needle) || version.contains(&needle)
    })
}

/// Sorts a slice of indices into `list` according to `sort`.
pub fn sort_indices<'a>(
    indices: &mut [usize],
    list: &[String],
    mut probe_of: impl FnMut(&str) -> Option<&'a super::gateway_info::Probe>,
    sort: GatewaySort,
) {
    match sort {
        GatewaySort::LastUsed => {}
        GatewaySort::Name => {
            indices.sort_by(|&a, &b| {
                let ua = &list[a];
                let ub = &list[b];
                let na = display_name(ua, probe_of(ua)).to_ascii_lowercase();
                let nb = display_name(ub, probe_of(ub)).to_ascii_lowercase();
                na.cmp(&nb).then_with(|| ua.cmp(ub))
            });
        }
        GatewaySort::Address => {
            indices.sort_by(|&a, &b| {
                let ua = &list[a];
                let ub = &list[b];
                let ha = super::gateway_info::address_host(ua)
                    .unwrap_or(ua)
                    .to_ascii_lowercase();
                let hb = super::gateway_info::address_host(ub)
                    .unwrap_or(ub)
                    .to_ascii_lowercase();
                ha.cmp(&hb).then_with(|| ua.cmp(ub))
            });
        }
        GatewaySort::Version => {
            indices.sort_by(|&a, &b| {
                let ua = &list[a];
                let ub = &list[b];
                let va = match probe_of(ua) {
                    Some(super::gateway_info::Probe::Known(info)) => info.short_version(),
                    _ => String::new(),
                };
                let vb = match probe_of(ub) {
                    Some(super::gateway_info::Probe::Known(info)) => info.short_version(),
                    _ => String::new(),
                };
                version_cmp(&vb, &va).then_with(|| {
                    let na = display_name(ua, probe_of(ua)).to_ascii_lowercase();
                    let nb = display_name(ub, probe_of(ub)).to_ascii_lowercase();
                    na.cmp(&nb)
                })
            });
        }
    }
}

/// Token inside a pre-release suffix (`beta.3b` -> `Str("beta")`, `Num(3)`, `Str("b")`).
#[derive(Debug, PartialEq, Eq)]
enum PreToken<'a> {
    Num(u64),
    Str(&'a str),
}

impl Ord for PreToken<'_> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self, other) {
            (Self::Num(a), Self::Num(b)) => a.cmp(b),
            (Self::Str(a), Self::Str(b)) => a.cmp(b),
            (Self::Num(_), Self::Str(_)) => std::cmp::Ordering::Less,
            (Self::Str(_), Self::Num(_)) => std::cmp::Ordering::Greater,
        }
    }
}

impl PartialOrd for PreToken<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

fn pre_tokens(pre: &str) -> Vec<PreToken<'_>> {
    let mut out = Vec::new();
    let bytes = pre.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'.' || b == b'-' {
            i += 1;
        } else if b.is_ascii_digit() {
            let start = i;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            let num = pre[start..i].parse::<u64>().unwrap_or(0);
            out.push(PreToken::Num(num));
        } else {
            let start = i;
            while i < bytes.len()
                && !bytes[i].is_ascii_digit()
                && bytes[i] != b'.'
                && bytes[i] != b'-'
            {
                i += 1;
            }
            out.push(PreToken::Str(&pre[start..i]));
        }
    }
    out
}

struct ParsedVersion<'a> {
    core: (u64, u64, u64),
    pre: Option<Vec<PreToken<'a>>>,
}

fn parse_semver(raw: &str) -> Option<ParsedVersion<'_>> {
    let clean = raw
        .trim()
        .split(|c: char| c == '+' || c.is_whitespace())
        .next()
        .unwrap_or("");
    let clean = clean
        .strip_prefix('v')
        .or_else(|| clean.strip_prefix('V'))
        .unwrap_or(clean);
    if clean.is_empty() || clean == "?" {
        return None;
    }
    let (core, pre) = match clean.split_once('-') {
        Some((c, p)) => (c, Some(pre_tokens(p))),
        None => (clean, None),
    };
    let mut nums = core.split('.');
    let major = nums.next()?.parse::<u64>().ok()?;
    let minor = nums.next().and_then(|s| s.parse::<u64>().ok()).unwrap_or(0);
    let patch = nums.next().and_then(|s| s.parse::<u64>().ok()).unwrap_or(0);
    Some(ParsedVersion {
        core: (major, minor, patch),
        pre,
    })
}

/// Compares two version strings (`v0.1.1-beta.3c` > `v0.1.1-beta.3b` > `v0.1.0`),
/// putting known versions above unknown/empty ones and stable releases above
/// pre-releases of the same `major.minor.patch`.
#[must_use]
pub fn version_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    match (parse_semver(a), parse_semver(b)) {
        (None, None) => std::cmp::Ordering::Equal,
        (Some(_), None) => std::cmp::Ordering::Greater,
        (None, Some(_)) => std::cmp::Ordering::Less,
        (
            Some(ParsedVersion {
                core: core_a,
                pre: pre_a,
            }),
            Some(ParsedVersion {
                core: core_b,
                pre: pre_b,
            }),
        ) => {
            core_a.cmp(&core_b).then_with(|| match (pre_a, pre_b) {
                (None, None) => std::cmp::Ordering::Equal,
                // Stable release > pre-release of the same core version.
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (Some(_), None) => std::cmp::Ordering::Less,
                (Some(ta), Some(tb)) => ta.cmp(&tb),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_page_served_by_its_gateway_names_that_gateway() {
        // The live deployment: https://gateway.baylee.app/play/ with no
        // `?gateway=`, so the client was configured with its own origin.
        assert_eq!(page_gateway(Some(PINNED), PINNED).as_deref(), Some(PINNED));
        // Another gateway serving the same build names itself, trailing
        // slash or not.
        assert_eq!(
            page_gateway(Some("https://play.example"), "https://play.example/").as_deref(),
            Some("https://play.example")
        );
        assert_eq!(
            page_gateway(Some("http://127.0.0.1:28766"), "http://127.0.0.1:28766").as_deref(),
            Some("http://127.0.0.1:28766")
        );
    }

    #[test]
    fn a_page_told_another_gateway_or_no_page_names_none() {
        // Native: there is no page.
        assert_eq!(page_gateway(None, PINNED), None);
        // `trunk serve` on :8080 with `?gateway=` (or a remembered one): the
        // page's origin is not the gateway, and is not offered as one.
        assert_eq!(
            page_gateway(Some("http://127.0.0.1:8080"), "http://127.0.0.1:28766"),
            None
        );
        // An opaque origin names no host, even when it is all there is.
        assert_eq!(page_gateway(Some("null"), "null"), None);
        assert_eq!(page_gateway(Some("file://"), "file://"), None);
        assert_eq!(page_gateway(Some("https://"), "https://"), None);
        assert_eq!(page_gateway(Some(""), ""), None);
    }

    #[test]
    fn the_page_gateway_joins_the_list_once_at_the_front() {
        let page = "https://play.example";
        let list = with_page(with_pinned(Vec::new()), Some(page));
        assert_eq!(list, vec![page.to_string()]);
        // Already saved: kept where it is, not doubled.
        let saved = with_pinned(vec!["http://10.0.0.2:28766".to_string(), page.to_string()]);
        assert_eq!(with_page(saved.clone(), Some(page)), saved);
        // The pinned gateway's own page changes nothing when already saved.
        let pinned = with_pinned(vec![PINNED.to_string()]);
        assert_eq!(with_page(pinned.clone(), Some(PINNED)), pinned);
        // No page, no change.
        assert_eq!(with_page(pinned.clone(), None), pinned);
    }

    #[test]
    fn a_page_served_by_a_gateway_starts_the_cursor_on_it() {
        let page = "https://play.example";
        // Wherever the uses put it, and in a debug build too.
        let list = vec![
            PINNED.to_string(),
            "http://10.0.0.2:28766".to_string(),
            page.to_string(),
        ];
        assert_eq!(starting_row(&list, true, Some(page)), Some(2));
        assert_eq!(starting_row(&list, false, Some(page)), Some(2));
        // The live page lands on the pinned row, as a release always did.
        assert_eq!(starting_row(&list, true, Some(PINNED)), Some(0));
        // Natively nothing changes: the release rule decides.
        assert_eq!(starting_row(&list, true, None), preselected(&list, true));
        assert_eq!(starting_row(&list, false, None), None);
    }

    /// The whole browser path as `LobbyState::new` runs it: page origin and
    /// configured address in, list and starting row out.
    #[test]
    fn a_fresh_browser_on_a_self_hosted_gateway_plays_there() {
        let origin = "https://table.example";
        let page = page_gateway(Some(origin), origin);
        let list = with_page(with_pinned(Vec::new()), page.as_deref());
        let row = starting_row(&list, true, page.as_deref());
        assert_eq!(row.map(|i| list[i].as_str()), Some(origin));
        // The same page told `?gateway=` elsewhere has no page gateway and starts empty.
        let page = page_gateway(Some(origin), "http://127.0.0.1:28766");
        let list = with_page(with_pinned(Vec::new()), page.as_deref());
        let row = starting_row(&list, true, page.as_deref());
        assert_eq!(row, None);
    }

    #[test]
    fn an_empty_list_stays_empty() {
        assert_eq!(with_pinned(Vec::new()), Vec::<String>::new());
    }

    #[test]
    fn a_saved_list_keeps_its_order_and_is_not_doubled() {
        let saved = vec!["http://127.0.0.1:28766".to_string(), PINNED.to_string()];
        assert_eq!(with_pinned(saved.clone()), saved);
        let without = vec!["http://127.0.0.1:28766".to_string()];
        assert_eq!(
            with_pinned(without),
            vec!["http://127.0.0.1:28766".to_string()]
        );
    }

    /// A list saved before the move of 30.10.2026 names the live gateway by
    /// its former address: it comes back under the pinned one, where the old
    /// row stood, once, and a second pass changes nothing.
    #[test]
    #[ignore = "the client still pins the old address; runs from the flip on (#402)"]
    fn a_former_address_of_the_live_gateway_is_read_as_the_pinned_one() {
        let old = KEYED;
        assert_eq!(moved(old), PINNED);
        assert_eq!(moved(&format!("{old}/")), PINNED);
        assert_eq!(moved("http://127.0.0.1:28766"), "http://127.0.0.1:28766");
        // A lookalike is someone else's gateway.
        assert_eq!(
            moved("https://baylee.acevik.de.example"),
            "https://baylee.acevik.de.example"
        );
        let saved = vec!["http://10.0.0.2:28766".to_string(), old.to_string()];
        let read = with_pinned(saved);
        assert_eq!(
            read,
            vec!["http://10.0.0.2:28766".to_string(), PINNED.to_string()]
        );
        assert_eq!(with_pinned(read.clone()), read);
        // Both spellings saved (a list synced from a new and an old device):
        // the pinned address once, where the first of them stood.
        let both = vec![
            old.to_string(),
            "http://10.0.0.2:28766".to_string(),
            PINNED.to_string(),
        ];
        assert_eq!(
            renamed(both),
            vec![PINNED.to_string(), "http://10.0.0.2:28766".to_string()]
        );
    }

    #[test]
    #[ignore = "the client still pins the old address; runs from the flip on (#402)"]
    fn the_live_gateway_is_the_same_by_either_address() {
        assert!(same(KEYED, PINNED));
        assert!(same(&format!("{PINNED}/"), KEYED));
        assert!(same("http://127.0.0.1:28766/", "http://127.0.0.1:28766"));
        assert!(!same(PINNED, "http://127.0.0.1:28766"));
        assert!(!same("https://baylee.acevik.de.example", PINNED));
    }

    /// Keys stay bound to the live gateway's first address: its new and its
    /// old spelling find the same vault and chat records; other gateways keep
    /// their own.
    #[test]
    #[ignore = "the client still pins the old address; runs from the flip on (#402)"]
    fn the_live_gateway_s_keys_stay_bound_to_its_first_address() {
        assert_eq!(keyed(PINNED), KEYED);
        assert_eq!(keyed(KEYED), KEYED);
        assert_eq!(keyed(&format!("{KEYED}/")), KEYED);
        assert_eq!(keyed("http://127.0.0.1:28766"), "http://127.0.0.1:28766");
    }

    /// The account's document keeps the live gateway under the address an
    /// installed client pins; reading it back gives the list as it was.
    #[test]
    #[ignore = "the client still pins the old address; runs from the flip on (#402)"]
    fn the_synced_list_keeps_the_live_gateway_under_its_first_address() {
        let list = vec![PINNED.to_string(), "http://10.0.0.2:28766".to_string()];
        let document = filed(&list);
        assert_eq!(
            document,
            vec![KEYED.to_string(), "http://10.0.0.2:28766".to_string()]
        );
        assert_eq!(filed(&document), document);
        assert_eq!(renamed(document), list);
    }

    /// `/play/` still served at the former address (the lobby's path, as
    /// `LobbyState::new` runs it): one row for the live gateway, the pinned
    /// one, and the cursor on it, where a guest kept on this browser is.
    #[test]
    #[ignore = "the client still pins the old address; runs from the flip on (#402)"]
    fn a_page_served_at_the_former_address_plays_at_the_pinned_row() {
        let old = KEYED;
        let page = page_gateway(Some(old), old).map(|page| moved(&page).to_string());
        assert_eq!(page.as_deref(), Some(PINNED));
        let list = with_page(with_pinned(vec![old.to_string()]), page.as_deref());
        assert_eq!(list, vec![PINNED.to_string()]);
        assert_eq!(starting_row(&list, false, page.as_deref()), Some(0));
    }

    /// An entry kept under the former address comes over; one kept under
    /// the pinned address already is never overwritten.
    #[test]
    #[ignore = "the client still pins the old address; runs from the flip on (#402)"]
    fn a_map_by_address_moves_a_former_entry_and_overwrites_nothing() {
        let mut map = std::collections::BTreeMap::from([
            (KEYED.to_string(), 1),
            ("http://10.0.0.2:28766".to_string(), 2),
        ]);
        rekeyed(&mut map);
        assert_eq!(
            map,
            std::collections::BTreeMap::from([
                (PINNED.to_string(), 1),
                ("http://10.0.0.2:28766".to_string(), 2),
            ])
        );
        let again = map.clone();
        rekeyed(&mut map);
        assert_eq!(map, again);
        let mut both =
            std::collections::BTreeMap::from([(KEYED.to_string(), 1), (PINNED.to_string(), 3)]);
        rekeyed(&mut both);
        assert_eq!(both.get(PINNED), Some(&3));
        assert_eq!(both.get(KEYED), Some(&1));
    }

    #[test]
    fn any_gateway_including_pinned_can_be_forgotten() {
        let mut list = with_pinned(vec![
            PINNED.to_string(),
            "http://10.0.0.2:28766".to_string(),
        ]);
        assert!(forget(&mut list, "http://10.0.0.2:28766"));
        assert_eq!(list, vec![PINNED.to_string()]);
        assert!(forget(&mut list, PINNED));
        assert!(list.is_empty());
        assert!(!forget(&mut list, PINNED));
    }

    #[test]
    fn only_a_release_preselects_and_only_the_pinned_row() {
        let list = with_pinned(vec![
            PINNED.to_string(),
            "http://127.0.0.1:28766".to_string(),
        ]);
        assert_eq!(preselected(&list, true), Some(0));
        assert_eq!(preselected(&list, false), None);
        assert_eq!(preselected(&["http://x.test".to_string()], true), None);
    }

    #[test]
    fn gateways_group_by_compatibility_and_availability() {
        use super::super::gateway_info::{GatewayInfo, Probe};
        let compatible = Probe::Known(GatewayInfo {
            name: Some("Sanctuary".to_string()),
            version: "0.2.0".to_string(),
            protocol_version: 3,
            view_version: 40,
            ..GatewayInfo::default()
        });
        let wrong_view = Probe::Known(GatewayInfo {
            name: Some("Staging".to_string()),
            version: "0.1.9".to_string(),
            protocol_version: 3,
            view_version: 39,
            ..GatewayInfo::default()
        });
        assert_eq!(GatewayGroup::of(None, 3, 40), GatewayGroup::Compatible);
        assert_eq!(
            GatewayGroup::of(Some(&Probe::Asking), 3, 40),
            GatewayGroup::Compatible
        );
        assert_eq!(
            GatewayGroup::of(Some(&compatible), 3, 40),
            GatewayGroup::Compatible
        );
        assert_eq!(
            GatewayGroup::of(Some(&wrong_view), 3, 40),
            GatewayGroup::Incompatible
        );
        assert_eq!(
            GatewayGroup::of(Some(&Probe::Older), 3, 40),
            GatewayGroup::Incompatible
        );
        assert_eq!(
            GatewayGroup::of(Some(&Probe::Silent), 3, 40),
            GatewayGroup::Offline
        );
        assert!(!GatewayGroup::Compatible.collapsed_by_default());
        assert!(GatewayGroup::Incompatible.collapsed_by_default());
        assert!(GatewayGroup::Offline.collapsed_by_default());
    }

    #[test]
    fn search_and_sort_cover_name_address_and_version() {
        use super::super::gateway_info::{GatewayInfo, Probe};
        let list = vec![
            "https://zeta.example".to_string(),
            "https://alpha.example".to_string(),
            PINNED.to_string(),
        ];
        let probes = std::collections::BTreeMap::from([
            (
                "https://zeta.example".to_string(),
                Probe::Known(GatewayInfo {
                    name: Some("Citadel".to_string()),
                    version: "0.3.0".to_string(),
                    protocol_version: 3,
                    view_version: 40,
                    ..GatewayInfo::default()
                }),
            ),
            (
                "https://alpha.example".to_string(),
                Probe::Known(GatewayInfo {
                    name: Some("Bastion".to_string()),
                    version: "0.1.0".to_string(),
                    protocol_version: 3,
                    view_version: 40,
                    ..GatewayInfo::default()
                }),
            ),
        ]);
        assert!(matches_query(&list[0], probes.get(&list[0]), "citadel 0.3"));
        assert!(!matches_query(&list[0], probes.get(&list[0]), "bastion"));
        assert!(matches_query(PINNED, None, "sanctuary"));

        let mut by_name = vec![0, 1, 2];
        sort_indices(&mut by_name, &list, |u| probes.get(u), GatewaySort::Name);
        // Bastion (1), Baylee Sanctuary (2), Citadel (0)
        assert_eq!(by_name, vec![1, 2, 0]);

        let mut by_addr = vec![0, 1, 2];
        sort_indices(&mut by_addr, &list, |u| probes.get(u), GatewaySort::Address);
        // alpha.example (1), baylee.acevik.de (2), zeta.example (0)
        assert_eq!(by_addr, vec![1, 2, 0]);

        let mut by_ver = vec![0, 1, 2];
        sort_indices(&mut by_ver, &list, |u| probes.get(u), GatewaySort::Version);
        // v0.3.0 (0), v0.1.0 (1), unprobed (2)
        assert_eq!(by_ver, vec![0, 1, 2]);
    }

    #[test]
    fn version_cmp_orders_prerelease_letter_suffixes_and_semver_cores() {
        use std::cmp::Ordering::{Greater, Less};
        assert_eq!(version_cmp("v0.1.1-beta.3c", "v0.1.1-beta.3b"), Greater);
        assert_eq!(version_cmp("v0.1.1-beta.3b", "v0.1.1-beta.3c"), Less);
        assert_eq!(version_cmp("v0.1.1-beta.3b", "v0.1.0-beta.9z"), Greater);
        assert_eq!(version_cmp("v0.1.0-beta.9z", "v0.1.1-beta.3b"), Less);
        assert_eq!(version_cmp("v0.1.1", "v0.1.1-beta.3c"), Greater);
        assert_eq!(version_cmp("v0.1.1-beta.10a", "v0.1.1-beta.3c"), Greater);
        assert_eq!(version_cmp("v0.1.0", ""), Greater);
        assert_eq!(version_cmp("v?", "v0.1.0"), Less);
    }
}
