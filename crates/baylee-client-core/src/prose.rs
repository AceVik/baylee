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
}
