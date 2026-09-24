//! Usernames: the name a player signs in with (#269).
//!
//! A username is private. It is shown to its owner and asked for at sign-in,
//! and nobody else ever sees it: the name other players see is the display
//! name with its tag (`Alice#af03`). A public login name would give away half
//! of every credential.
//!
//! One rule for both ends. The gateway links this crate and so does the
//! client, which checks a name before sending it so that it can say what is
//! wrong with it; the gateway checks again, because a client is only a
//! client.

use unicode_normalization::UnicodeNormalization;

/// The fewest characters a username has.
pub const USERNAME_MIN: usize = 3;

/// The most characters a username has.
pub const USERNAME_MAX: usize = 24;

/// The characters that may stand between letters and digits.
pub const SEPARATORS: [char; 3] = ['_', '-', '.'];

/// A username that passed [`username`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Username {
    /// The name as its owner is shown it: what was typed, in Unicode's
    /// compatibility form (NFKC), so a full-width `Ａｌｉｃｅ` is `Alice`.
    pub shown: String,
    /// What the name is unique under and looked up by: `shown` in lower
    /// case, so `Alice` and `alice` are one name.
    pub key: String,
}

/// Why a username was refused, each for its own message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsernameFault {
    /// A control character, or one that is invisible or turns the direction
    /// of the text: such a name can read as another one, letter for letter.
    Invisible,
    /// A character other than an ASCII letter, a digit or a separator,
    /// after normalisation.
    Character,
    /// Fewer than [`USERNAME_MIN`] or more than [`USERNAME_MAX`] characters.
    Length,
    /// Begins or ends with a separator.
    Edge,
    /// Two separators in a row (`a..b`, `a.-b`).
    Doubled,
}

/// `typed` as a username, or why it is not one.
///
/// In this order, so that the message names the first thing to fix:
/// invisible characters are refused before anything else, because
/// normalisation keeps them and they are what a look-alike is made of; then
/// the name is normalised (NFKC) and has to be ASCII letters, digits and
/// [`SEPARATORS`], [`USERNAME_MIN`] to [`USERNAME_MAX`] of them, beginning and
/// ending on a letter or a digit, with no two separators side by side.
///
/// Nothing is trimmed: a space is a character like any other, and the
/// client trims what it sends.
///
/// # Errors
///
/// The first [`UsernameFault`] the name has.
pub fn username(typed: &str) -> Result<Username, UsernameFault> {
    if typed.chars().any(invisible) {
        return Err(UsernameFault::Invisible);
    }
    let shown: String = typed.nfkc().collect();
    if !shown
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || SEPARATORS.contains(&c))
    {
        return Err(UsernameFault::Character);
    }
    // ASCII by now, so a byte is a character.
    if !(USERNAME_MIN..=USERNAME_MAX).contains(&shown.len()) {
        return Err(UsernameFault::Length);
    }
    let plain = |b: Option<&u8>| b.is_some_and(u8::is_ascii_alphanumeric);
    if !plain(shown.as_bytes().first()) || !plain(shown.as_bytes().last()) {
        return Err(UsernameFault::Edge);
    }
    if shown
        .as_bytes()
        .windows(2)
        .any(|pair| pair.iter().all(|b| !b.is_ascii_alphanumeric()))
    {
        return Err(UsernameFault::Doubled);
    }
    Ok(Username {
        key: shown.to_ascii_lowercase(),
        shown,
    })
}

/// The nearest thing to `text` that keeps [`username`]'s character rules,
/// for a name made on a player's behalf rather than typed.
///
/// Folded (NFKC) like a typed name; every character that is still not an
/// ASCII letter, digit or separator becomes `-`; a run of separators is
/// squeezed to its first; none is left at either end; and it is cut to
/// [`USERNAME_MAX`]. What it does not do is make the name long enough: the
/// result may be shorter than [`USERNAME_MIN`], or empty, and the caller
/// decides what to add.
#[must_use]
pub fn nearest(text: &str) -> String {
    let mut name = String::new();
    for c in text.nfkc() {
        let c = if c.is_ascii_alphanumeric() || SEPARATORS.contains(&c) {
            c
        } else {
            '-'
        };
        let separator = !c.is_ascii_alphanumeric();
        let after_one = name
            .chars()
            .next_back()
            .is_none_or(|last| !last.is_ascii_alphanumeric());
        if separator && after_one {
            continue;
        }
        name.push(c);
    }
    name.truncate(USERNAME_MAX);
    while name.ends_with(SEPARATORS) {
        name.pop();
    }
    name
}

/// A control character, or a format character that draws nothing or turns
/// the direction of what follows it (Unicode's `Cf` that a name could hide).
fn invisible(c: char) -> bool {
    c.is_control()
        || matches!(
            c,
            '\u{00AD}'
                | '\u{061C}'
                | '\u{180E}'
                | '\u{200B}'..='\u{200F}'
                | '\u{202A}'..='\u{202E}'
                | '\u{2060}'..='\u{2064}'
                | '\u{2066}'..='\u{206F}'
                | '\u{FEFF}'
                | '\u{FFF9}'..='\u{FFFB}'
        )
}

#[cfg(test)]
mod tests {
    use super::{Username, UsernameFault, username};

    fn shown(typed: &str) -> String {
        username(typed).map(|name| name.shown).unwrap_or_default()
    }

    #[test]
    fn a_plain_name_is_its_own_shown_form_and_its_lower_case_is_its_key() {
        assert_eq!(
            username("Alice.B-2_c"),
            Ok(Username {
                shown: "Alice.B-2_c".into(),
                key: "alice.b-2_c".into(),
            })
        );
        assert_eq!(
            username("ALICE").map(|name| name.key),
            username("alice").map(|name| name.key),
            "one name in two cases is one name"
        );
    }

    /// NFKC folds what only looks different into what it looks like, so a
    /// look-alike is the same name, taken, rather than a second one.
    #[test]
    fn compatibility_forms_are_the_letters_they_stand_for() {
        assert_eq!(shown("Ａｌｉｃｅ"), "Alice", "full-width");
        assert_eq!(shown("ﬁnn"), "finn", "a ligature");
        assert_eq!(shown("\u{212A}elvin"), "Kelvin", "the Kelvin sign");
        assert_eq!(shown("ab\u{FF3F}cd"), "ab_cd", "a full-width low line");
    }

    #[test]
    fn invisible_and_direction_characters_are_refused_before_anything_else() {
        for typed in [
            "ali\u{200B}ce",
            "ali\u{202E}ec",
            "\u{2066}alice",
            "alice\u{200E}",
            "al\u{061C}ice",
            "ali\u{00AD}ce",
            "\u{FEFF}alice",
            "ali\nce",
            "ali\u{7}ce",
            // Invisible *and* too short: the invisible character is the
            // thing to fix first.
            "a\u{200D}",
        ] {
            assert_eq!(username(typed), Err(UsernameFault::Invisible), "{typed:?}");
        }
    }

    #[test]
    fn only_ascii_letters_digits_and_three_separators_are_allowed() {
        for typed in [
            "ali ce",
            "alice@home",
            "Alice#af03",
            "jürgen",
            "Алиса",
            "a+b+c",
            "日本語の名前",
        ] {
            assert_eq!(username(typed), Err(UsernameFault::Character), "{typed:?}");
        }
    }

    #[test]
    fn three_to_twenty_four_characters() {
        assert_eq!(username("ab"), Err(UsernameFault::Length));
        assert!(username("abc").is_ok());
        assert!(username(&"a".repeat(24)).is_ok());
        assert_eq!(username(&"a".repeat(25)), Err(UsernameFault::Length));
        assert_eq!(username(""), Err(UsernameFault::Length));
        // Counted after folding: three full-width letters are three.
        assert!(username("ＡＢＣ").is_ok());
    }

    /// What a name made on a player's behalf looks like: the characters a
    /// typed one may have, and only where it may have them.
    #[test]
    fn the_nearest_name_keeps_the_rules_and_leaves_the_length_to_the_caller() {
        use super::nearest;
        assert_eq!(nearest("alice+baylee"), "alice-baylee");
        assert_eq!(nearest("Jürgen.Müller"), "J-rgen.M-ller");
        assert_eq!(nearest("ａｌｉｃｅ"), "alice");
        assert_eq!(nearest("..a..b--c__"), "a.b-c");
        assert_eq!(nearest("a.-_b"), "a.b");
        assert_eq!(nearest("+++"), "");
        assert_eq!(nearest("x"), "x");
        assert_eq!(nearest("ali\u{200B}ce"), "ali-ce");
        // Cut, and a separator the cut left at the end goes too.
        let long = format!("{}.b", "a".repeat(23));
        assert_eq!(nearest(&long), "a".repeat(23));
        for text in [
            "alice+baylee",
            "Jürgen.Müller",
            "..a..b--c__",
            "日本語の名前だよ",
        ] {
            let near = nearest(text);
            assert!(
                near.len() < 3 || username(&near).is_ok(),
                "{text:?} came out as {near:?}"
            );
        }
    }

    #[test]
    fn a_separator_stands_only_between_letters_and_digits() {
        for typed in [".alice", "alice.", "_alice", "alice-", "-a-"] {
            assert_eq!(username(typed), Err(UsernameFault::Edge), "{typed:?}");
        }
        for typed in ["a..b", "a.-b", "a__b", "al-_ce"] {
            assert_eq!(username(typed), Err(UsernameFault::Doubled), "{typed:?}");
        }
        assert!(username("a.b-c_d").is_ok());
    }
}
