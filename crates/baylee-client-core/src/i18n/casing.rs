//! A status line as a sentence, where it is shown.

use std::borrow::Cow;

/// `text` with a capital first letter, for a line that stands on its own.
///
/// The lobby's status lines are written to be read inside other sentences too, so
/// about two hundred of them start small ("signed in"), and a toast or a banner is
/// where that shows. Here is the one place that says so, and `Lobby::status` keeps
/// what it was told.
///
/// Only a word that is nothing but small letters is touched. A line may start with
/// a value, and a value is not ours to change: an address (`http://…`,
/// `example.org`), a handle (`alice#0012`), a number, a name written as `macOS`.
#[must_use]
pub fn sentence_case(text: &str) -> Cow<'_, str> {
    let word = text.split(char::is_whitespace).next().unwrap_or_default();
    let word = word.trim_end_matches(|c: char| !c.is_alphanumeric());
    let small = |c: char| c.is_lowercase() || c == '-' || c == '\'';
    let mut letters = text.chars();
    let (Some(first), false) = (letters.next(), word.is_empty()) else {
        return Cow::Borrowed(text);
    };
    let mut capital = first.to_uppercase();
    match (capital.next(), capital.next()) {
        // One letter into one: "ß" has no capital of its own here.
        (Some(capital), None) if first.is_lowercase() && word.chars().all(small) => {
            Cow::Owned(capital.to_string() + letters.as_str())
        }
        _ => Cow::Borrowed(text),
    }
}

#[cfg(test)]
mod tests {
    use super::sentence_case;

    #[test]
    fn a_line_that_starts_small_starts_with_a_capital() {
        assert_eq!(sentence_case("signed in"), "Signed in");
        assert_eq!(sentence_case("saved."), "Saved.");
        assert_eq!(sentence_case("sign-in failed"), "Sign-in failed");
        assert_eq!(sentence_case("signed in — welcome"), "Signed in — welcome");
        assert_eq!(sentence_case("überprüft"), "Überprüft");
    }

    /// What is already right is handed back as it stands, not copied.
    #[test]
    fn a_line_with_its_capital_is_left_alone() {
        for said in ["Signed in", "", " ", "— so", "…"] {
            assert!(
                matches!(sentence_case(said), std::borrow::Cow::Borrowed(_)),
                "{said:?}"
            );
        }
    }

    /// A value at the start of a line is not a word of ours.
    #[test]
    fn a_value_at_the_start_of_a_line_is_not_touched() {
        for said in [
            "http://127.0.0.1:28790 is already in your gateways.",
            "example.org is not answering",
            "localhost:28766 is not answering",
            "alice#0012 left",
            "3 of 5 seated",
            "macOS needs a newer build",
            "iPhone",
            "ßeta",
        ] {
            assert_eq!(sentence_case(said), said);
        }
    }
}
