//! The vocabulary every rule in this crate is written in: sentences, mana
//! symbols as Scryfall spells them and as some printings misspell them, and
//! the part of a sentence a cost can stand in.

use std::borrow::Cow;

/// The sentences a printed text is made of.
///
/// Scryfall prints one rules sentence per line and separates paragraphs
/// with a blank one. Blank lines are dropped and every line is trimmed, so
/// the index this yields is an index into **this** iterator and not into
/// [`str::lines`].
///
/// This is the split codegen counts the English Oracle with when it writes
/// the ability-line table, and the split a client resolves that index
/// against. The two ends cannot see each other, which is why the three lines
/// live here: a split that drifted would not fail loudly, it would point at
/// the sentence beside the right one.
pub fn sentences(text: &str) -> impl Iterator<Item = &str> {
    text.lines().map(str::trim).filter(|l| !l.is_empty())
}

/// How many sentences a printed text is made of.
#[must_use]
pub fn sentence_count(text: &str) -> usize {
    sentences(text).count()
}

/// Mana symbols as Scryfall writes them, whatever the printing's data says.
///
/// Two misspellings reach the catalog from Scryfall's localized data, and
/// both would otherwise make a sentence look like a different ability:
///
/// - a hybrid symbol with its parts in lower case and in parentheses —
///   `{(}w/b)}` (Fetid Heath, fic) and `{(u/r)}` (Cascade Bluffs, eoc) —
///   which is `{W/B}` / `{U/R}`;
/// - several symbols merged into one pair of braces — `{2BB}` (Ifnir
///   Deadlands, ecc), `{1R}` (Kavaron, eoe), `{UU}` — which is one symbol per
///   letter after the generic number: `{2}{B}{B}`.
///
/// A symbol that is already one of Scryfall's (`{C}`, `{X}`, `{10}`, `{2/U}`,
/// `{G/P}`) is left alone, and so is everything outside braces.
#[must_use]
pub fn repair_braces(text: &str) -> Cow<'_, str> {
    if !text.contains('{') {
        return Cow::Borrowed(text);
    }
    let mut out = String::with_capacity(text.len() + 8);
    let mut changed = false;
    let mut rest = text;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        rest = &rest[open..];
        if let Some((symbol, used)) = garbled_hybrid(rest) {
            out.push_str(&symbol);
            rest = &rest[used..];
            changed = true;
            continue;
        }
        let Some(close) = rest[1..].find('}').map(|c| c + 1) else {
            break;
        };
        let inner = &rest[1..close];
        if let Some(split) = merged(inner) {
            out.push_str(&split);
            changed = true;
        } else {
            out.push_str(&rest[..=close]);
        }
        rest = &rest[close + 1..];
    }
    out.push_str(rest);
    if changed {
        Cow::Owned(out)
    } else {
        Cow::Borrowed(text)
    }
}

/// `{(}w/b)}` or `{(w/b)}` at the start of `at`, as `{W/B}` and its length.
fn garbled_hybrid(at: &str) -> Option<(String, usize)> {
    let body = at
        .strip_prefix("{(}")
        .map(|b| (b, 3))
        .or_else(|| at.strip_prefix("{(").map(|b| (b, 2)));
    let (body, prefix) = body?;
    let mut chars = body.chars();
    let (a, slash, b) = (chars.next()?, chars.next()?, chars.next()?);
    if slash != '/'
        || !is_colour_letter(a)
        || !is_colour_letter(b)
        || !chars.as_str().starts_with(")}")
    {
        return None;
    }
    let symbol = format!("{{{}/{}}}", a.to_ascii_uppercase(), b.to_ascii_uppercase());
    Some((symbol, prefix + 3 + 2))
}

fn is_colour_letter(c: char) -> bool {
    matches!(c, 'w' | 'u' | 'b' | 'r' | 'g')
}

/// `2BB` as `{2}{B}{B}`, when the braces held more than one symbol.
fn merged(inner: &str) -> Option<String> {
    let letters_at = inner.find(|c: char| !(c.is_ascii_digit() || c == 'X'))?;
    let (number, letters) = inner.split_at(letters_at);
    if letters.is_empty() || !letters.chars().all(|c| "WUBRGC".contains(c)) {
        return None;
    }
    if number.is_empty() && letters.len() < 2 {
        return None;
    }
    let mut out = String::new();
    if !number.is_empty() {
        out.push('{');
        out.push_str(number);
        out.push('}');
    }
    for c in letters.chars() {
        out.push('{');
        out.push(c);
        out.push('}');
    }
    Some(out)
}

/// The mana and tap symbols a text prints, as a sorted multiset.
///
/// Braces are repaired first ([`repair_braces`]), and a symbol is compared
/// in upper case, so `{2BB}` and `{2}{B}{B}` are the same key.
#[must_use]
pub fn symbols(text: &str) -> Vec<String> {
    let text = repair_braces(text);
    let mut out = Vec::new();
    let mut rest: &str = &text;
    while let Some(open) = rest.find('{') {
        rest = &rest[open + 1..];
        let Some(close) = rest.find(['{', '}']) else {
            break;
        };
        if rest.as_bytes()[close] == b'}' && close > 0 {
            out.push(format!("{{{}}}", rest[..close].to_uppercase()));
            rest = &rest[close + 1..];
        } else {
            rest = &rest[close..];
        }
    }
    out.sort_unstable();
    out
}

/// Whether every symbol of `part` is in `whole`, counting repeats.
pub(crate) fn is_submultiset(part: &[String], whole: &[String]) -> bool {
    let mut whole = whole.iter();
    part.iter().all(|symbol| {
        whole
            .by_ref()
            .find(|c| *c >= symbol)
            .is_some_and(|c| c == symbol)
    })
}

/// Characters that open a span a cost can never be inside: reminder text
/// (CR 207.2) and a quoted ability another object gains.
const LEAD_ENDS: [char; 7] = ['(', '（', '"', '„', '“', '«', '「'];

/// The part of a sentence a cost can be printed in: everything before its
/// first reminder or quotation.
///
/// Chromatic Lantern's German first line, `Länder, die du kontrollierst,
/// haben „{T}: Erzeuge …"`, has a colon, and it is the colon of the ability
/// the lands gain rather than one of the Lantern's costs. Bonesplitter's
/// `Ausrüsten {1} ({1}: Lege …)` has one inside its reminder. Neither is a
/// cost, and a cut at either would draw half a sentence in the cost column.
///
/// A parenthesis inside braces opens nothing: Fetid Heath's fic printing
/// spells its hybrid `{(}w/b)}`, and ending the lead there left a cost of
/// `{`.
pub(crate) fn lead(sentence: &str) -> &str {
    let mut braces = 0_u32;
    for (at, c) in sentence.char_indices() {
        match c {
            '{' => braces += 1,
            '}' => braces = braces.saturating_sub(1),
            _ if braces == 0 && LEAD_ENDS.contains(&c) => return &sentence[..at],
            _ => {}
        }
    }
    sentence
}

/// Where the cost colon of a sentence is, and how wide it is.
///
/// The first `:` or `：` in the [`lead`], and not at its very start. A French
/// printing writes `{W}, {T} : Engagez …` with a space before it, Flame
/// Spirit's German 6ed printing `{R}:Der Flammengeist …` with none after it,
/// and a Japanese one the full-width `：`; the earliest colon is the cost's
/// in all three, because a cost cannot contain one.
pub(crate) fn cost_colon(sentence: &str) -> Option<(usize, usize)> {
    let lead = lead(sentence);
    let (at, c) = lead.char_indices().find(|&(_, c)| c == ':' || c == '：')?;
    (at > 0).then_some((at, c.len_utf8()))
}

/// The cost part of a sentence — up to its cost colon, or its whole [`lead`]
/// when it has none — which is what [`verify`](crate::verify) keys on.
pub(crate) fn head(sentence: &str) -> &str {
    match cost_colon(sentence) {
        Some((at, _)) => &sentence[..at],
        None => lead(sentence),
    }
}

/// Whether a sentence is reminder text and nothing else.
///
/// Its first character opens a parenthesis and the parenthesis it opens is
/// closed by its last one. `(Mauern können nicht angreifen.){T}: …` (Vine
/// Trellis, 8ed) opens with one and is not reminder-only, which a test on
/// the first and last characters alone would have said it was.
pub(crate) fn reminder_only(sentence: &str) -> bool {
    let mut chars = sentence.char_indices();
    let Some((_, first)) = chars.next() else {
        return false;
    };
    if first != '(' && first != '（' {
        return false;
    }
    let mut depth = 1_u32;
    for (at, c) in chars {
        match c {
            '(' | '（' => depth += 1,
            ')' | '）' => {
                depth -= 1;
                if depth == 0 {
                    return at + c.len_utf8() == sentence.len();
                }
            }
            _ => {}
        }
    }
    false
}

/// A text with its reminders, spacing and case taken out, for asking whether
/// two texts are the same words.
///
/// `keep_reminders` exists for one case: a card whose whole text is a
/// reminder (a basic land's `({T}: Add {G}.)`) folds to nothing without it,
/// and two nothings are equal whatever language they were in.
pub(crate) fn fold(text: &str, keep_reminders: bool) -> String {
    let text = repair_braces(text);
    let mut out = String::with_capacity(text.len());
    let mut depth = 0_u32;
    let mut space = false;
    for c in text.chars() {
        if !keep_reminders {
            match c {
                '(' | '（' => {
                    depth += 1;
                    continue;
                }
                ')' | '）' if depth > 0 => {
                    depth -= 1;
                    continue;
                }
                _ if depth > 0 => continue,
                _ => {}
            }
        }
        if c.is_whitespace() || c == '|' {
            space = !out.is_empty();
            continue;
        }
        if space {
            out.push(' ');
            space = false;
        }
        out.extend(c.to_lowercase());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The blank line between two paragraphs is a separator, not a
    /// sentence — an index that counted it would be off by one for every
    /// ability below the first paragraph.
    #[test]
    fn a_blank_line_between_paragraphs_is_not_a_sentence() {
        let oracle = "Flying\n\nWhen this creature enters, draw a card.\n";
        let got: Vec<&str> = sentences(oracle).collect();
        assert_eq!(got, ["Flying", "When this creature enters, draw a card."]);
        assert_eq!(sentence_count(oracle), 2);
    }

    /// Trailing whitespace is Scryfall's, not the card's.
    #[test]
    fn a_sentence_carries_no_surrounding_whitespace() {
        assert_eq!(sentences("  {T}: Add {G}.  ").next(), Some("{T}: Add {G}."));
    }

    #[test]
    fn a_garbled_hybrid_is_the_hybrid_it_means() {
        assert_eq!(repair_braces("{(}w/b)}, {T}: X"), "{W/B}, {T}: X");
        assert_eq!(repair_braces("{(u/r)}, {T}: X"), "{U/R}, {T}: X");
    }

    #[test]
    fn merged_symbols_are_split_after_the_generic_number() {
        assert_eq!(repair_braces("{2BB}, {T}"), "{2}{B}{B}, {T}");
        assert_eq!(repair_braces("{1R}"), "{1}{R}");
        assert_eq!(repair_braces("Add {UU}, {UR}"), "Add {U}{U}, {U}{R}");
        assert_eq!(repair_braces("{XR}"), "{X}{R}");
    }

    /// The other branch: every symbol Scryfall itself prints stays as it is,
    /// and a text that needed nothing is not copied.
    #[test]
    fn a_symbol_scryfall_prints_is_left_alone() {
        let text = "{C}{X}{10}{2/U}{G/P}{W/U}{T}{Q}{S}{E}{½}";
        assert!(matches!(repair_braces(text), Cow::Borrowed(_)));
        assert!(matches!(repair_braces("no braces"), Cow::Borrowed(_)));
        assert_eq!(repair_braces("{unclosed"), "{unclosed");
    }

    #[test]
    fn symbols_are_a_sorted_multiset_over_the_repaired_text() {
        assert_eq!(symbols("{2BB}, {T}"), ["{2}", "{B}", "{B}", "{T}"]);
        assert_eq!(symbols("{T}, {2}{B}{B}"), ["{2}", "{B}", "{B}", "{T}"]);
        assert_eq!(symbols("no symbols"), Vec::<String>::new());
        assert_eq!(symbols("{t}"), ["{T}"]);
    }

    #[test]
    fn a_submultiset_counts_repeats() {
        let whole = symbols("{1}{B}{B}{T}");
        assert!(is_submultiset(&symbols("{B}{T}"), &whole));
        assert!(is_submultiset(&[], &whole));
        assert!(!is_submultiset(&symbols("{B}{B}{B}"), &whole));
        assert!(!is_submultiset(&symbols("{R}"), &whole));
    }

    #[test]
    fn a_cost_colon_is_the_first_one_before_any_reminder_or_quote() {
        fn at(s: &str) -> Option<&str> {
            cost_colon(s).map(|(at, _)| &s[..at])
        }
        assert_eq!(
            at("{1}, {T}, opfere dieses Artefakt: Ziehe eine Karte."),
            Some("{1}, {T}, opfere dieses Artefakt")
        );
        assert_eq!(at("{R}:Der Flammengeist erhält +1/+0."), Some("{R}"));
        assert_eq!(
            at("{W}, {T} : Engagez la créature ciblée."),
            Some("{W}, {T} ")
        );
        assert_eq!(at("+2：プレイヤー１人を対象とする。"), Some("+2"));
        assert_eq!(at("Ausrüsten {1} ({1}: Lege diese Karte an.)"), None);
        assert_eq!(
            at("Länder, die du kontrollierst, haben „{T}: Erzeuge {C}.\""),
            None
        );
        assert_eq!(at(": leading"), None);
        assert_eq!(at("{(}w/b)}, {T}: Erzeuge {W}{W}."), Some("{(}w/b)}, {T}"));
    }

    #[test]
    fn reminder_only_means_one_parenthesis_around_everything() {
        assert!(reminder_only("(Mauern können nicht angreifen)"));
        assert!(reminder_only("（このクリーチャーは攻撃できない。）"));
        assert!(reminder_only("({T}: Add {G} or {W}.)"));
        assert!(!reminder_only(
            "(Mauern können nicht angreifen.){T}: Erhöhe deinen Manavorrat um {G}."
        ));
        assert!(!reminder_only("(a) and (b)"));
        assert!(!reminder_only("Defender (This creature can't attack.)"));
        assert!(!reminder_only(""));
    }

    #[test]
    fn a_fold_drops_reminders_spacing_and_case() {
        assert_eq!(
            fold("Station (Tap another creature.)\n12+ | {1R}, {T}", false),
            fold("Station\n12+ | {1}{R}, {T}", false)
        );
        assert_eq!(fold("({T}: Add {G}.)", false), "");
        assert_eq!(fold("({T}: Add {G}.)", true), "({t}: add {g}.)");
    }
}
