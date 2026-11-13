//! A person's mark where they have no picture: a monogram on a tinted disc.
//!
//! The same person wears the same letters and the same tint everywhere they
//! are named, in this client and on Baylee Central's website, so the rule
//! here is Central's own (`web/src/chat/conversations.ts`, `initials` and
//! `hue`; `web/src/styles/chat.css`, `.chat-avatar`) and not one of ours:
//!
//! - the letters are the first of a name, or of its first two words, with
//!   the `#tag` left off;
//! - the tint is one of six, chosen by a hash over the account's tag written
//!   in **decimal** (Central's tag is a number; the game's handles spell it
//!   in hex, which [`tint_of`] turns back);
//! - the disc is 24 % of the tint over Central's panel, the letters 70 % of
//!   it over its title ink, the rim the tint at 34 %.
//!
//! Everything here is a pure decision; `baylee-client` draws it
//! (`shellkit::avatar`).

/// The six tints a monogram can wear, in Central's order (`data-hue` 0–5).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Tint {
    /// `--violet`.
    Violet,
    /// `--gold`.
    Gold,
    /// `--ok`.
    Green,
    /// `--chart-reports`.
    Blue,
    /// `--chart-crashes`.
    Red,
    /// `--untested`.
    Cyan,
}

/// Central's panel, which the disc's tint is mixed over (`--panel-solid`).
const PANEL: [u8; 3] = [0x17, 0x27, 0x41];

/// Central's title ink, which the letters' tint is mixed over (`--title`).
const TITLE: [u8; 3] = [0xff, 0xf7, 0xe4];

impl Tint {
    /// Every tint, in Central's order: index `n` is `data-hue="n"`.
    pub const ALL: [Self; 6] = [
        Self::Violet,
        Self::Gold,
        Self::Green,
        Self::Blue,
        Self::Red,
        Self::Cyan,
    ];

    /// The tint itself, as sRGB bytes.
    #[must_use]
    pub const fn srgb(self) -> [u8; 3] {
        match self {
            Self::Violet => [0xae, 0xbf, 0xff],
            Self::Gold => [0xe3, 0xc0, 0x7e],
            Self::Green => [0x7f, 0xd3, 0xa1],
            Self::Blue => [0x6f, 0x8a, 0xe8],
            Self::Red => [0xe0, 0x66, 0x5a],
            Self::Cyan => [0x8f, 0xcf, 0xe0],
        }
    }

    /// The disc under the letters: 24 % of the tint over the panel.
    #[must_use]
    pub fn ground(self) -> [f32; 3] {
        mix(self.srgb(), PANEL, 0.24)
    }

    /// The letters: 70 % of the tint over the title ink.
    #[must_use]
    pub fn ink(self) -> [f32; 3] {
        mix(self.srgb(), TITLE, 0.70)
    }

    /// The rim: the tint itself, drawn at [`RIM_ALPHA`].
    #[must_use]
    pub fn rim(self) -> [f32; 3] {
        self.srgb().map(|c| f32::from(c) / 255.0)
    }
}

/// How strongly the rim's tint is drawn over whatever the disc stands on.
pub const RIM_ALPHA: f32 = 0.34;

/// `share` of `tint` over `under`, mixed in sRGB as CSS's
/// `color-mix(in srgb, …)` mixes, as sRGB components in 0–1.
fn mix(tint: [u8; 3], under: [u8; 3], share: f32) -> [f32; 3] {
    let channel =
        |i: usize| (f32::from(tint[i]) * share + f32::from(under[i]) * (1.0 - share)) / 255.0;
    [channel(0), channel(1), channel(2)]
}

/// What stands in for a person's picture.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Monogram {
    /// One or two capital letters, or `?`.
    pub letters: String,
    /// The disc's tint.
    pub tint: Tint,
}

/// The monogram of the person a handle names (`Alice#af03`), or of a bare
/// name with no tag, whose tint is then hashed from the name.
#[must_use]
pub fn monogram(handle: &str) -> Monogram {
    let (name, tag) = match handle.rsplit_once('#') {
        Some((name, tag)) if is_tag(tag) => (name, tag),
        _ => (handle, ""),
    };
    Monogram {
        letters: letters(name),
        tint: if tag.is_empty() {
            tint_of_seed(name)
        } else {
            tint_of(tag)
        },
    }
}

/// Whether `tag` is a handle's tag: lowercase hex and nothing else.
fn is_tag(tag: &str) -> bool {
    !tag.is_empty() && tag.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// A name's letters: the first of the name, or of its first two words (split
/// at spaces, underscores, hyphens and full stops), in capitals; `?` for a
/// name with none. A handle's `#tag` is left off first.
#[must_use]
pub fn letters(name: &str) -> String {
    let name = match name.rsplit_once('#') {
        Some((before, tag)) if is_tag(tag) => before,
        _ => name,
    };
    let mut words = name
        .split(|c: char| c.is_whitespace() || matches!(c, '_' | '-' | '.'))
        .filter(|word| !word.is_empty());
    let first = words.next().and_then(|word| word.chars().next());
    let second = words.next().and_then(|word| word.chars().next());
    let letters: String = first
        .into_iter()
        .chain(second)
        .flat_map(char::to_uppercase)
        .collect();
    if letters.is_empty() {
        "?".to_string()
    } else {
        letters
    }
}

/// The tint of the account whose tag a handle spells `tag` (hex, as the
/// gateway writes it): Central's hash over the same number in decimal. A tag
/// that is no number is hashed as written.
#[must_use]
pub fn tint_of(tag: &str) -> Tint {
    match u32::from_str_radix(tag, 16) {
        Ok(number) => tint_of_seed(&number.to_string()),
        Err(_) => tint_of_seed(tag),
    }
}

/// Central's `hue(seed)`: `h = h·31 + code point`, wrapping at 32 bits, over
/// the seed's characters, modulo the six tints.
#[must_use]
pub fn tint_of_seed(seed: &str) -> Tint {
    let hash = seed
        .chars()
        .fold(0u32, |h, c| h.wrapping_mul(31).wrapping_add(u32::from(c)));
    Tint::ALL[(hash % 6) as usize]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Central's `initials`, case for case: two words give two letters, one
    /// gives one, the tag is never a word, and nothing gives a question mark.
    #[test]
    fn the_letters_are_the_first_of_a_name_or_of_its_first_two_words() {
        for (name, want) in [
            ("Alice", "A"),
            ("alice#af03", "A"),
            ("Mia Wallace", "MW"),
            ("mia_wallace#0001", "MW"),
            ("jean-luc.picard", "JL"),
            ("  spaced   out  ", "SO"),
            ("ärger", "Ä"),
            ("ßtraße", "SS"),
            ("#af03", "?"),
            ("", "?"),
            ("___", "?"),
            ("one two three", "OT"),
            ("Tag#NotATag", "T"),
        ] {
            assert_eq!(letters(name), want, "{name:?}");
        }
    }

    /// Central's `hue` over `String(tag)`: values computed by hand from
    /// `h = (h * 31 + codePoint) >>> 0`, so a drift from the website's rule
    /// fails here and not as two colours for one person.
    #[test]
    fn the_tint_is_centrals_hash_over_the_tag_in_decimal() {
        // "1" → 49 % 6 = 1; "10" → (49·31 + 48) = 1567 % 6 = 1;
        // "255" → ((50·31 + 53)·31 + 53) = 49746 % 6 = 0.
        assert_eq!(tint_of_seed("1"), Tint::Gold);
        assert_eq!(tint_of_seed("10"), Tint::Gold);
        assert_eq!(tint_of_seed("255"), Tint::Violet);
        // The handle spells the same number in hex.
        assert_eq!(tint_of("0001"), tint_of_seed("1"));
        assert_eq!(tint_of("000a"), tint_of_seed("10"));
        assert_eq!(tint_of("00ff"), tint_of_seed("255"));
        assert_eq!(monogram("Alice#00ff").tint, Tint::Violet);
        assert_eq!(monogram("Alice#00ff").letters, "A");
        // The hash wraps at 32 bits as `>>> 0` does, and does not panic.
        let long = "9".repeat(64);
        let _ = tint_of_seed(&long);
        // Every tint is reachable, so no person is stuck with one of five.
        let mut seen = std::collections::BTreeSet::new();
        for tag in 0..64u32 {
            seen.insert(tint_of_seed(&tag.to_string()) as u8);
        }
        assert_eq!(seen.len(), 6);
    }

    /// A name with no tag still has a tint, and always the same one.
    #[test]
    fn a_bare_name_is_tinted_by_its_own_letters() {
        assert_eq!(monogram("House AI").tint, tint_of_seed("House AI"));
        assert_eq!(monogram("House AI").letters, "HA");
        assert_eq!(monogram("House AI"), monogram("House AI"));
    }

    /// The mixes are CSS's `color-mix(in srgb, …)`: 24 % of violet over the
    /// panel, 70 % over the title ink. The letters stay readable on their
    /// disc for every tint (WCAG contrast of at least 4.5).
    #[test]
    fn the_disc_and_its_letters_are_mixed_as_the_website_mixes_them() {
        let ground = Tint::Violet.ground();
        let want = [
            (174.0 * 0.24 + 23.0 * 0.76) / 255.0,
            (191.0 * 0.24 + 39.0 * 0.76) / 255.0,
            (255.0 * 0.24 + 65.0 * 0.76) / 255.0,
        ];
        for (got, want) in ground.iter().zip(want) {
            assert!((got - want).abs() < 1e-5);
        }
        let luminance = |srgb: [f32; 3]| {
            let linear = |c: f32| {
                if c <= 0.040_45 {
                    c / 12.92
                } else {
                    ((c + 0.055) / 1.055).powf(2.4)
                }
            };
            0.2126 * linear(srgb[0]) + 0.7152 * linear(srgb[1]) + 0.0722 * linear(srgb[2])
        };
        for tint in Tint::ALL {
            let (ink, ground) = (luminance(tint.ink()), luminance(tint.ground()));
            let contrast = (ink + 0.05) / (ground + 0.05);
            assert!(contrast >= 4.5, "{tint:?}: {contrast}");
        }
    }
}
