//! Searching a library, and how long an exile lasts.

use super::{CounterKind, Filter};

/// What a reflexive triggered ability waits for (CR 603.12): an event
/// that the resolution creating it has already caused.
///
/// "When you do" names the action printed directly before it, and the
/// event is what makes the sentence a trigger rather than an `if`. The
/// enum has the variants this pool needs. Grist's −2 ("you may sacrifice a
/// creature. When you do, …") needs `Sacrificed(&Filter)`, and Agatha's
/// Soul Cauldron ("when a creature card is exiled this way") needs
/// `Exiled(&Filter)`. Both come with their cards, and each brings its own
/// action clause to the placement lint.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ReflexiveEvent {
    /// "Sacrifice it. When you do, …" / "Then you may sacrifice this land.
    /// When you do, …": this resolution sacrificed its own source.
    SacrificedThis,
    /// "You may exile it. When you do, …" (The Balrog of Moria, whose dies
    /// trigger exiles the card from the graveyard): this resolution exiled
    /// its own source, by `Effect::ExileSource`.
    ExiledThis,
}

/// Where a searched card goes.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum SearchDest {
    /// Into your hand.
    Hand,
    /// Onto the battlefield (optionally tapped).
    Battlefield,
    /// On top of your library.
    TopOfLibrary,
}

/// Where one card found by [`Effect::SearchLibrary`] goes.
///
/// Cards are matched to finds positionally, so a search that produces fewer
/// cards than it allows fills the finds from the front — the order in the
/// slice is the order the card text names them.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Find {
    /// Where this card goes.
    pub dest: SearchDest,
    /// Whether it enters tapped (battlefield only).
    pub tapped: bool,
    /// A counter it enters with (battlefield only): Neoform's "put that card
    /// onto the battlefield with an additional +1/+1 counter on it".
    ///
    /// Put on through `replacement::put_counters` once the card has arrived,
    /// the door `GraveyardToBattlefield`'s counter takes, so a doubler has
    /// its say (CR 614.16). Written with [`Find::with_counter`].
    pub counter: Option<(CounterKind, u16)>,
    /// A fork on the card found: one that matches the filter goes where the
    /// inner find says instead. Archdruid's Charm, "Put it onto the
    /// battlefield tapped if it's a land card. Otherwise, put it into your
    /// hand", is `Find::HAND.when_matching(&Filter::LAND,
    /// &Find::BATTLEFIELD_TAPPED)`. Asked of the card once it is chosen, as
    /// it is in the library. Written with [`Find::when_matching`].
    pub instead_if: Option<(&'static Filter, &'static Find)>,
    /// This find takes every further card the search finds as well, so the
    /// search may find as many cards as match: The World Tree's "search
    /// your library for any number of God cards, put them onto the
    /// battlefield" is one repeating find in an optional search. Only the
    /// last find of a search repeats. Written with [`Find::any_number`].
    pub repeats: bool,
}

impl Find {
    /// Into your hand.
    pub const HAND: Self = Self {
        dest: SearchDest::Hand,
        tapped: false,
        counter: None,
        instead_if: None,
        repeats: false,
    };
    /// Onto the battlefield, untapped (Nature's Lore, a fetchland).
    pub const BATTLEFIELD: Self = Self {
        dest: SearchDest::Battlefield,
        tapped: false,
        counter: None,
        instead_if: None,
        repeats: false,
    };
    /// Onto the battlefield tapped (Rampant Growth, Evolving Wilds).
    pub const BATTLEFIELD_TAPPED: Self = Self {
        dest: SearchDest::Battlefield,
        tapped: true,
        counter: None,
        instead_if: None,
        repeats: false,
    };
    /// On top of your library (a tutor that does not draw).
    pub const TOP_OF_LIBRARY: Self = Self {
        dest: SearchDest::TopOfLibrary,
        tapped: false,
        counter: None,
        instead_if: None,
        repeats: false,
    };

    /// The same find, entering with `n` counters of `kind` on it — "with an
    /// additional +1/+1 counter on it" (Neoform).
    #[must_use]
    pub const fn with_counter(self, kind: CounterKind, n: u16) -> Self {
        Self {
            counter: Some((kind, n)),
            ..self
        }
    }

    /// The same find, except that a found card matching `filter` goes
    /// where `then` says ([`Find::instead_if`]).
    #[must_use]
    pub const fn when_matching(self, filter: &'static Filter, then: &'static Self) -> Self {
        Self {
            instead_if: Some((filter, then)),
            ..self
        }
    }

    /// The same find, for "any number of" cards ([`Find::repeats`]).
    #[must_use]
    pub const fn any_number(self) -> Self {
        Self {
            repeats: true,
            ..self
        }
    }
}

/// The event an "exile … until …" sentence waits for (CR 610.3), read by
/// [`Effect::ExileLinked`].
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ExileUntil {
    /// "until this creature leaves the battlefield" (Werefox Bodyguard):
    /// the source, as the object it was when the ability triggered or was
    /// activated. A blink ends it too, since the permanent that comes back
    /// is a new object (CR 400.7).
    SourceLeavesBattlefield,
    /// "until an opponent becomes the monarch" (Palace Jailer): an opponent
    /// of the player who controlled the exiling ability, whoever controls
    /// the source later.
    OpponentBecomesMonarch,
}
