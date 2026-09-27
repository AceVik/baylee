//! The saved gateway list and the one entry in it nobody can remove.
//!
//! The live gateway, "Baylee Sanctuary", is where a player who has never
//! typed an address plays. It is always in the list, and forgetting it is
//! refused here rather than only hidden in the interface: a button taken
//! away is one path, a rule in the list is every path.

/// The live gateway's address, spelled as a saved address is: scheme and
/// host, no trailing slash.
pub const PINNED: &str = "https://baylee.acevik.de";

/// What the live gateway calls itself (its `BAYLEE_GATEWAY_NAME`).
pub const PINNED_NAME: &str = "Baylee Sanctuary";

/// Whether an address is the pinned one.
#[must_use]
pub fn is_pinned(url: &str) -> bool {
    url == PINNED
}

/// The list with the pinned gateway in it: a saved list keeps its order and
/// gains the pinned address at the front if it lacked it.
#[must_use]
pub fn with_pinned(mut list: Vec<String>) -> Vec<String> {
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
        // The live deployment: https://baylee.acevik.de/play/ with no
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
