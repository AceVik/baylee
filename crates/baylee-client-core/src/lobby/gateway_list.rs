//! The saved gateway list and the one entry in it nobody can remove.
//!
//! The live gateway, "Baylee Sanctuary", is where a player who has never
//! typed an address plays. It is always in the list, and forgetting it is
//! refused here rather than only hidden in the interface: a button taken
//! away is one path, a rule in the list is every path.

/// The live gateway's address, spelled as a saved address is: scheme and
/// host, no trailing slash.
pub const PINNED: &str = "https://gateway.baylee.app";

/// The live gateway's former addresses (moved 30.10.2026). A client
/// installed before the move saved one of these wherever it now saves
/// [`PINNED`]; [`moved`] reads them as [`PINNED`]. The old host still serves
/// the gateway as an alias, so an address missed here still plays, but under
/// another name than the pinned row's.
pub const FORMER: &[&str] = &["https://baylee.acevik.de"];

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

/// The list with the pinned gateway in it: a saved list keeps its order,
/// reads a former address of the live gateway as the pinned one
/// ([`renamed`]), and gains the pinned address at the front if it lacked it.
#[must_use]
pub fn with_pinned(list: Vec<String>) -> Vec<String> {
    let mut list = renamed(list);
    if !list.iter().any(|url| is_pinned(url)) {
        list.insert(0, PINNED.to_string());
    }
    list
}

/// Removes an address from the list. Answers whether anything was removed;
/// the pinned address never is.
pub fn forget(list: &mut Vec<String>, url: &str) -> bool {
    if is_pinned(url) {
        return false;
    }
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
        assert_eq!(list, vec![page.to_string(), PINNED.to_string()]);
        // Already saved: kept where it is, not doubled.
        let saved = with_pinned(vec!["http://10.0.0.2:28766".to_string(), page.to_string()]);
        assert_eq!(with_page(saved.clone(), Some(page)), saved);
        // The pinned gateway's own page changes nothing.
        let pinned = with_pinned(Vec::new());
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
        // The same page told `?gateway=` elsewhere keeps today's behaviour:
        // no page gateway, the cursor on the pinned row.
        let page = page_gateway(Some(origin), "http://127.0.0.1:28766");
        let list = with_page(with_pinned(Vec::new()), page.as_deref());
        let row = starting_row(&list, true, page.as_deref());
        assert_eq!(row.map(|i| list[i].as_str()), Some(PINNED));
    }

    #[test]
    fn an_empty_list_gains_the_pinned_gateway() {
        assert_eq!(with_pinned(Vec::new()), vec![PINNED.to_string()]);
    }

    #[test]
    fn a_saved_list_keeps_its_order_and_is_not_doubled() {
        let saved = vec!["http://127.0.0.1:28766".to_string(), PINNED.to_string()];
        assert_eq!(with_pinned(saved.clone()), saved);
        let without = vec!["http://127.0.0.1:28766".to_string()];
        assert_eq!(
            with_pinned(without),
            vec![PINNED.to_string(), "http://127.0.0.1:28766".to_string()]
        );
    }

    /// A list saved before the move of 30.10.2026 names the live gateway by
    /// its former address: it comes back under the pinned one, where the old
    /// row stood, once, and a second pass changes nothing.
    #[test]
    fn a_former_address_of_the_live_gateway_is_read_as_the_pinned_one() {
        let old = FORMER[0];
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
    fn the_live_gateway_is_the_same_by_either_address() {
        assert!(same(FORMER[0], PINNED));
        assert!(same(&format!("{PINNED}/"), FORMER[0]));
        assert!(same("http://127.0.0.1:28766/", "http://127.0.0.1:28766"));
        assert!(!same(PINNED, "http://127.0.0.1:28766"));
        assert!(!same("https://baylee.acevik.de.example", PINNED));
    }

    /// Keys stay bound to the live gateway's first address: its new and its
    /// old spelling find the same vault and chat records; other gateways keep
    /// their own.
    #[test]
    fn the_live_gateway_s_keys_stay_bound_to_its_first_address() {
        assert_eq!(keyed(PINNED), KEYED);
        assert_eq!(keyed(KEYED), KEYED);
        assert_eq!(keyed(&format!("{KEYED}/")), KEYED);
        assert_eq!(keyed("http://127.0.0.1:28766"), "http://127.0.0.1:28766");
        assert!(FORMER.contains(&KEYED));
    }

    /// The account's document keeps the live gateway under the address an
    /// installed client pins; reading it back gives the list as it was.
    #[test]
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
    fn a_page_served_at_the_former_address_plays_at_the_pinned_row() {
        let old = FORMER[0];
        let page = page_gateway(Some(old), old).map(|page| moved(&page).to_string());
        assert_eq!(page.as_deref(), Some(PINNED));
        let list = with_page(with_pinned(vec![old.to_string()]), page.as_deref());
        assert_eq!(list, vec![PINNED.to_string()]);
        assert_eq!(starting_row(&list, false, page.as_deref()), Some(0));
    }

    /// An entry kept under the former address comes over; one kept under
    /// the pinned address already is never overwritten.
    #[test]
    fn a_map_by_address_moves_a_former_entry_and_overwrites_nothing() {
        let mut map = std::collections::BTreeMap::from([
            (FORMER[0].to_string(), 1),
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
            std::collections::BTreeMap::from([(FORMER[0].to_string(), 1), (PINNED.to_string(), 3)]);
        rekeyed(&mut both);
        assert_eq!(both.get(PINNED), Some(&3));
        assert_eq!(both.get(FORMER[0]), Some(&1));
    }

    #[test]
    fn the_pinned_gateway_cannot_be_forgotten() {
        let mut list = with_pinned(vec!["http://10.0.0.2:28766".to_string()]);
        assert!(
            !forget(&mut list, PINNED),
            "forgetting the pinned gateway is refused"
        );
        assert!(list.iter().any(|url| is_pinned(url)));
        assert!(forget(&mut list, "http://10.0.0.2:28766"));
        assert_eq!(list, vec![PINNED.to_string()]);
    }

    #[test]
    fn only_a_release_preselects_and_only_the_pinned_row() {
        let list = with_pinned(vec!["http://127.0.0.1:28766".to_string()]);
        assert_eq!(preselected(&list, true), Some(0));
        assert_eq!(preselected(&list, false), None);
        assert_eq!(preselected(&["http://x.test".to_string()], true), None);
    }
}
