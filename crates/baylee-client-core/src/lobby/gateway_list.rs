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

#[cfg(test)]
mod tests {
    use super::*;

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
