use super::Phrase;
use super::lang::Lang;

impl Phrase {
    /// This phrase with its placeholders filled in, left to right.
    ///
    /// `{0}` is replaced by the first argument, `{1}` by the second, and a
    /// placeholder with no argument is left standing rather than swallowed —
    /// a visible `{2}` is a bug report; a silently missing number is a
    /// sentence that means something else.
    #[must_use]
    pub fn fill(self, lang: Lang, args: &[&str]) -> String {
        let mut text = self.text(lang).to_string();
        for (index, arg) in args.iter().enumerate() {
            text = text.replace(&format!("{{{index}}}"), arg);
        }
        text
    }

    /// What to say about a username the rule refused, one sentence per
    /// fault, so the player is told the thing to fix.
    #[must_use]
    pub fn username_fault(fault: baylee_protocol::names::UsernameFault) -> Self {
        use baylee_protocol::names::UsernameFault;
        match fault {
            UsernameFault::Invisible => Self::UsernameInvisible,
            UsernameFault::Character => Self::UsernameCharacters,
            UsernameFault::Length => Self::UsernameLength,
            UsernameFault::Edge => Self::UsernameEdge,
            UsernameFault::Doubled => Self::UsernameDoubled,
        }
    }

    /// The form of a counted sentence that `n` things ask for.
    ///
    /// Both languages split in the same place — exactly one against anything
    /// else — so the number picks the sentence and the language never has to
    /// be asked. **Zero takes the plural**, which is both languages again:
    /// "Choose up to 0 cards", "Wähle bis zu 0 Karten".
    ///
    /// This exists because the file said `card(s)` and `Karte(n)` out loud in
    /// five places, and the sheet's own typography greys a bracketed aside
    /// ([`crate::prose::bracketed`]) — so a broken plural was drawn as an
    /// editorial remark, in grey, next to the number it disagreed with. The
    /// repair is not a suffix: German wants a relative clause here
    /// ("Karte, die nach unten geht" against "Karten, die nach unten gehen"),
    /// and the verb inside it agrees too. Only a whole second literal can say
    /// that, which is why a counted phrase is written twice rather than
    /// assembled.
    #[must_use]
    pub const fn counted(n: usize, one: Self, many: Self) -> Self {
        if n == 1 { one } else { many }
    }

    /// The placeholders this phrase carries, as their indices.
    ///
    /// Used by the test that keeps the languages in step: word order is the
    /// translator's business and `{0}` may move anywhere, but a `{0}` that is
    /// not there at all is a name, a count or a reason that never reaches the
    /// player.
    #[must_use]
    pub fn slots(self, lang: Lang) -> Vec<usize> {
        let text = self.text(lang);
        let mut found: Vec<usize> = (0..10)
            .filter(|i| text.contains(&format!("{{{i}}}")))
            .collect();
        found.sort_unstable();
        found
    }
}
