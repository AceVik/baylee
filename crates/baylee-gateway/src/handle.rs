//! `Alice#af03`: a display name and the number that tells two Alices apart.
//!
//! The name is not unique and the tag is. That is the whole arrangement, and
//! it is Blizzard's rather than Discord's: Discord's four digits were unique
//! *per name*, so `Alice#0001` and `Bob#0001` both existed and the pair was
//! the identity. A tag that is unique on its own is a user number, and a user
//! number that renders in four digits runs out at the 65 537th account.
//!
//! So the rendering is **variable-length hex, padded to four**: `#0001` for
//! the first account, `#af03` for the 44 803rd, `#10000` for the one after
//! sixty-five thousand five hundred and thirty-six. Four digits is what a
//! player expects to see and the width nothing needs to plan for; growing
//! past it costs a character and no decision. Hex rather than decimal because
//! the owner asked for it and because it is shorter, and lowercase because
//! `#AF03` and `#af03` must not look like two accounts — parsing accepts
//! either and rendering only ever produces one.
//!
//! A lookup always carries the `#`. `Alice#af03` finds the account, `#af03`
//! finds it without knowing the name, and a bare `af03` finds nothing —
//! because `af03` is a legal display name and a lookup that guessed between
//! the two would answer a different question depending on who had registered.

/// The separator, which is exactly why [`crate::auth::valid_display_name`]
/// refuses it inside a name.
pub const SEP: char = '#';

/// How wide a tag is drawn before it has to grow.
const PAD: usize = 4;

/// A tag as a player reads it: lowercase hex, at least four digits.
#[must_use]
pub fn tag_text(tag: i32) -> String {
    // A tag is an identity column and starts at 1, so the negative half of
    // `i32` is unreachable. `unsigned_abs` rather than a cast because a row
    // that somehow held a negative tag should render oddly, not panic in
    // the middle of drawing a lobby.
    let unsigned = tag.unsigned_abs();
    let width = PAD;
    format!("{unsigned:0width$x}")
}

/// The whole handle, as it is drawn on a seat and in a roster.
#[must_use]
pub fn handle(display_name: &str, tag: i32) -> String {
    format!("{display_name}{SEP}{}", tag_text(tag))
}

/// Read a tag out of what a player typed, or out of what they pasted.
///
/// Accepts `Alice#af03`, `#af03` and either case, and refuses anything with
/// no separator, a tag that is not hex, an empty tag, and one that does not
/// fit the column.
#[must_use]
pub fn parse_tag(typed: &str) -> Option<i32> {
    let typed = typed.trim();
    let (_name, tag) = typed.rsplit_once(SEP)?;
    if tag.is_empty() || !tag.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    i32::try_from(u32::from_str_radix(tag, 16).ok()?).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tag_is_four_hex_digits_until_it_cannot_be() {
        assert_eq!(tag_text(1), "0001");
        assert_eq!(tag_text(44803), "af03");
        assert_eq!(tag_text(0xffff), "ffff");
        // The account after the sixty-five thousand five hundred and
        // thirty-sixth. Discord's discriminator had nowhere to go here;
        // this one grows a digit and every reader keeps working, because
        // nothing parses a fixed width.
        assert_eq!(tag_text(0x1_0000), "10000");
        assert_eq!(tag_text(i32::MAX), "7fffffff");
    }

    #[test]
    fn a_handle_survives_the_round_trip() {
        for tag in [1, 255, 44803, 0xffff, 0x1_0000, i32::MAX] {
            let drawn = handle("Alice", tag);
            assert_eq!(parse_tag(&drawn), Some(tag), "{drawn}");
        }
    }

    #[test]
    fn a_tag_alone_finds_an_account_and_a_bare_word_does_not() {
        assert_eq!(parse_tag("#af03"), Some(44803));
        assert_eq!(parse_tag("Alice#af03"), Some(44803));
        // Pasted from somewhere that shouted it.
        assert_eq!(parse_tag("Alice#AF03"), Some(44803));
        assert_eq!(parse_tag("  Alice#af03  "), Some(44803));

        // `af03` is a legal display name. Guessing between a name and a tag
        // would make the answer depend on who had registered.
        assert_eq!(parse_tag("af03"), None);
        assert_eq!(parse_tag("Alice"), None);
        assert_eq!(parse_tag("Alice#"), None);
        assert_eq!(parse_tag("Alice#zz"), None);
        // Wider than the column it would be compared against.
        assert_eq!(parse_tag("Alice#ffffffff"), None);
    }

    #[test]
    fn nothing_renders_a_tag_two_ways() {
        // Upper case is accepted on the way in and never produced on the
        // way out, so a name copied out of the lobby is the one that was
        // there.
        assert_eq!(tag_text(0xabcd), "abcd");
        assert_eq!(
            tag_text(parse_tag("x#ABCD").expect("upper case parses")),
            "abcd"
        );
    }
}
