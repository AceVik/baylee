//! Reading a line of interface prose for the parts that are asides.
//!
//! One question, asked by the prompt slip and answered here rather than in
//! the renderer: which stretches of a sentence are in brackets. A player
//! reading "Pass priority (Space)" is being told two things at once — what
//! the sheet is asking, and how to answer it without the pointer — and only
//! the first is the sentence. Drawing the second in the same ink makes the
//! line longer to read for no gain, so the slip greys it.
//!
//! It lives here, and not beside the drawing, because it is a decision about
//! text: it has an answer that can be wrong, and a test can say so.

/// The runs of `text`, each flagged `true` when it is a bracketed aside.
///
/// The brackets themselves belong to the aside — they are punctuation the
/// aside brought with it, and a grey aside between black brackets reads as a
/// mistake.
///
/// Borrows throughout and allocates nothing: a slip is rebuilt whenever the
/// view changes, and this runs once per line on it.
///
/// An **unclosed** bracket is not an aside. `"Discard down to 7 ("` is a
/// sentence that was cut off, or a card whose name contains a bracket, and
/// greying the whole tail of a line because of one stray character is a
/// worse failure than leaving it black.
#[must_use]
pub fn bracketed(text: &str) -> Bracketed<'_> {
    Bracketed { rest: text }
}

/// The iterator [`bracketed`] returns.
#[derive(Clone, Debug)]
pub struct Bracketed<'a> {
    /// What has not been handed out yet.
    rest: &'a str,
}

impl<'a> Iterator for Bracketed<'a> {
    type Item = (&'a str, bool);

    fn next(&mut self) -> Option<Self::Item> {
        if self.rest.is_empty() {
            return None;
        }
        // An aside runs from the bracket that opens it to the first one that
        // closes it. Nesting is not a thing interface prose does, and reading
        // it as one flat span keeps the scan linear.
        if self.rest.starts_with('(') {
            if let Some(end) = self.rest.find(')') {
                let (aside, tail) = self.rest.split_at(end + 1);
                self.rest = tail;
                return Some((aside, true));
            }
            return Some((std::mem::take(&mut self.rest), false));
        }
        // Plain text up to the next bracket that *does* close — an opening
        // bracket with nothing after it is not the start of anything.
        match self
            .rest
            .find('(')
            .filter(|open| self.rest[*open..].contains(')'))
        {
            Some(open) => {
                let (plain, tail) = self.rest.split_at(open);
                self.rest = tail;
                Some((plain, false))
            }
            None => Some((std::mem::take(&mut self.rest), false)),
        }
    }
}

/// A name reduced to what alphabetising and searching it should ignore.
///
/// Case, first — a list that filed *Zombie* before *ancestral* is not a list
/// — and then the accents, because `str::cmp` is **byte** order and every
/// accented Latin letter sits above `z` in it. A German graveyard sorted by
/// name put *Ätherfluss* after *Zombie*, at the bottom of the list, which is
/// the alphabet of no language at all. `ß` is the one that needs saying
/// aloud: lowercasing it leaves it exactly where it was (it *is* the lower
/// case), so it has to be *replaced* with `ss`, which is also the pair a
/// player types when their keyboard has no `ß`.
///
/// It folds to a **base letter**, not to a spelling-out: `ä` is `a`, as
/// German dictionary order has it (DIN 5007-1), rather than the `ae` a phone
/// book would want. The two ligatures are the exception and expand, because
/// `æ` and `œ` have no single base letter to fall back to.
///
/// # What it deliberately does not reach
///
/// Latin only. The catalog serves Japanese, Korean, Chinese, Russian, Hebrew
/// and Arabic too, and those come out unchanged — which is right for
/// Cyrillic, where code-point order *is* the alphabet, and simply not
/// something a table of characters can fix for Chinese, whose dictionary
/// order is by radical and stroke. Getting those right means a collator and
/// a locale, which is a dependency and a decision, not a fold. A list in one
/// of them is in a stable, repeatable order that is not its reader's; a list
/// in a Latin script is now in its reader's.
#[must_use]
pub fn sort_key(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars().flat_map(char::to_lowercase) {
        match c {
            'à'..='å' => out.push('a'),
            'æ' => out.push_str("ae"),
            'ç' => out.push('c'),
            'è'..='ë' => out.push('e'),
            'ì'..='ï' => out.push('i'),
            'ñ' => out.push('n'),
            'ò'..='ö' | 'ø' => out.push('o'),
            'œ' => out.push_str("oe"),
            'ß' => out.push_str("ss"),
            'ù'..='ü' => out.push('u'),
            'ý' | 'ÿ' => out.push('y'),
            plain => out.push(plain),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::bracketed;

    fn runs(text: &str) -> Vec<(&str, bool)> {
        bracketed(text).collect()
    }

    #[test]
    fn a_line_with_no_brackets_is_one_plain_run() {
        assert_eq!(runs("Pass priority"), vec![("Pass priority", false)]);
        assert_eq!(runs(""), Vec::new());
    }

    #[test]
    fn the_brackets_belong_to_the_aside_they_open() {
        assert_eq!(
            runs("Pass priority (Space)"),
            vec![("Pass priority ", false), ("(Space)", true)]
        );
        assert_eq!(
            runs("(Space) passes"),
            vec![("(Space)", true), (" passes", false)]
        );
    }

    #[test]
    fn several_asides_in_one_line_are_each_their_own_run() {
        assert_eq!(
            runs("Attack (A) or decline (D) now"),
            vec![
                ("Attack ", false),
                ("(A)", true),
                (" or decline ", false),
                ("(D)", true),
                (" now", false),
            ]
        );
    }

    /// The whole reason the scan looks ahead for the closing bracket instead
    /// of greying from the opening one to the end of the line.
    #[test]
    fn an_unclosed_bracket_greys_nothing() {
        assert_eq!(
            runs("Discard down to 7 ("),
            vec![("Discard down to 7 (", false)]
        );
        assert_eq!(runs("("), vec![("(", false)]);
        // The closed one is still an aside; only the stray tail is plain.
        assert_eq!(
            runs("Keep (7 cards) or ("),
            vec![("Keep ", false), ("(7 cards)", true), (" or (", false),]
        );
    }

    /// Rebuilding the line from its runs has to give the line back, whatever
    /// the brackets did — the slip draws these spans and nothing else.
    #[test]
    fn the_runs_always_spell_the_line_they_came_from() {
        for line in [
            "",
            "(",
            ")",
            "()",
            "a(b)c",
            ")stray( closing",
            "Sacrifice a creature (you control) (again)",
            "Nachziehen (2 Karten)",
        ] {
            let rebuilt: String = bracketed(line).map(|(run, _)| run).collect();
            assert_eq!(rebuilt, line, "runs of {line:?}");
        }
    }

    /// The wart that only shows once the names are not English.
    #[test]
    fn an_accented_name_sorts_where_its_alphabet_puts_it() {
        let mut names = vec!["Zombie", "Ätherfluss", "Aetherling", "Brutalizer"];
        names.sort_by_key(|n| super::sort_key(n));
        assert_eq!(
            names,
            ["Aetherling", "Ätherfluss", "Brutalizer", "Zombie"],
            "byte order files every accent above z"
        );
        // The counter-test, because a key that changed nothing would also
        // pass a list that happened to be in order: this is what it fixed.
        let mut raw = vec!["Zombie", "Ätherfluss", "Aetherling"];
        raw.sort_unstable();
        assert_eq!(raw, ["Aetherling", "Zombie", "Ätherfluss"]);
    }

    /// `ß` is the one a lowercasing pass walks straight past.
    #[test]
    fn the_sharp_s_is_replaced_rather_than_lowercased() {
        assert_eq!(super::sort_key("Straße"), "strasse");
        assert_eq!(super::sort_key("STRASSE"), "strasse", "the pair one types");
        assert_eq!(super::sort_key("STRAẞE"), "strasse", "the capital of it");
        assert_eq!("ß".to_lowercase(), "ß", "which is why a fold is needed");
    }

    /// Each letter reduces to its own base, and nothing else moves.
    #[test]
    fn the_fold_reaches_the_letters_the_catalog_prints() {
        assert_eq!(
            super::sort_key("Àáâãäå Çç Èéêë Ìíîï Ñ Òóôõöø Ùúûü Ýÿ"),
            "aaaaaa cc eeee iiii n oooooo uuuu yy"
        );
        assert_eq!(
            super::sort_key("Æther Œuvre"),
            "aether oeuvre",
            "no base letter to fall back to"
        );
        // Not Latin, so not folded — and stable, which is the whole claim.
        for other in ["稲妻", "Молния", "라이트닝"] {
            assert_eq!(super::sort_key(other), other.to_lowercase());
        }
    }
}
