/// A language the interface speaks.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Hash)]
pub enum Lang {
    /// English.
    #[default]
    En,
    /// German.
    De,
}

impl Lang {
    /// Every language, in the order a picker offers them.
    pub const ALL: [Self; 2] = [Self::En, Self::De];

    /// The language a stored code names.
    ///
    /// Anything unrecognised is English rather than an error: the code comes
    /// from a settings file, a query string or an account, and a client that
    /// refused to start over one would be worse than one that speaks English.
    /// A regional code (`de-DE`, `en_GB`) is read by its first part, because
    /// the catalog's languages are plain two-letter codes and a player who
    /// wrote one out in full meant the language.
    #[must_use]
    pub fn of(code: &str) -> Self {
        // Compared without folding into a new string: dozens of systems ask
        // this every frame, and a lowercased copy was an allocation each.
        let base = code.split(['-', '_']).next().unwrap_or_default();
        if base.eq_ignore_ascii_case("de") {
            Self::De
        } else {
            Self::En
        }
    }

    /// The code this language is stored and requested under.
    #[must_use]
    pub fn code(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::De => "de",
        }
    }

    /// What the language calls itself. Never "German" in an English list: a
    /// player looking for their own language is looking for their own word
    /// for it.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::En => "English",
            Self::De => "Deutsch",
        }
    }

    /// The next language round the ring — one button rather than a menu,
    /// which is what two languages deserve.
    #[must_use]
    pub fn next(self) -> Self {
        let at = Self::ALL.iter().position(|l| *l == self).unwrap_or(0);
        Self::ALL[(at + 1) % Self::ALL.len()]
    }
}
