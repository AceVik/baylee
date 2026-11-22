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
    /// This resolution exiled one of its targeted cards matching the filter.
    Exiled(&'static crate::Filter),
    /// "You may sacrifice [the permanent that granted this ability]. When
    /// you do, …": this resolution sacrificed that exact incarnation.
    SacrificedGrantor,
    /// "You may sacrifice a creature. When you do, …" (Grist, the Hunger
    /// Tide's −2): this resolution's controller sacrificed a permanent the
    /// filter matched as it left, by effect.
    Sacrificed(&'static crate::Filter),
    /// "You may exert ~ as it attacks. When you do, …" (CR 701.43d): this
    /// resolution exerted its own source on the battlefield via
    /// [`crate::Effect::ExertSource`]. Appended to preserve replay identities.
    ExertedThis,
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
    /// Into the searched card's owner's graveyard.
    Graveyard,
    /// Into public exile, owned by the searched card's owner.
    Exile,
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
    /// Into public exile.
    pub const EXILE: Self = Self {
        dest: SearchDest::Exile,
        ..Self::HAND
    };
    /// Into its owner's graveyard.
    pub const GRAVEYARD: Self = Self {
        dest: SearchDest::Graveyard,
        ..Self::HAND
    };
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

/// Domain iterated by [`super::Effect::RepeatEach`] (CR 101.4, CR 608.2h).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum RepeatDomain {
    /// Each living player in APNAP turn order (CR 101.4), binding
    /// `PlayerRel::EventPlayer` to the current player on each pass.
    EachPlayer,
    /// Each living opponent of the resolving controller in APNAP turn order
    /// (CR 101.4), binding `PlayerRel::EventPlayer` on each pass.
    EachOpponent,
    /// Each battlefield permanent matching the filter as the loop begins
    /// (CR 608.2h, ordered by controller APNAP then battlefield order), binding
    /// `TargetSpec::EventObject`, LKI power/toughness/mana-value, and
    /// `PlayerRel::EventPlayer` (its controller) on each pass.
    EachMatchingPermanent(&'static Filter),
    /// A bounded count evaluated once as the loop begins (CR 608.2h).
    Times(super::Amount),
}

/// Which zone [`super::Effect::ChooseCard`] or [`super::Effect::ChooseCardEx`]
/// offers selectable objects/cards from.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ChooseZone {
    /// Permanents on the battlefield (`filter` narrows controller/owner/qualities).
    Battlefield,
    /// Cards in the graveyard(s) of `PlayerRel`.
    Graveyard(super::PlayerRel),
    /// Cards in the hand(s) of `PlayerRel`. Appended to preserve replay identities.
    Hand(super::PlayerRel),
    /// Cards in the exile zone(s) of `PlayerRel`. Appended to preserve replay
    /// identities.
    Exile(super::PlayerRel),
    /// Cards in exile linked to the resolving ability's source (`CR 406.6`,
    /// `CR 607.2a`). Appended to preserve replay identities.
    LinkedExile,
    /// Objects/cards bound in `SubjectContext::found` earlier in this resolution
    /// in whichever zone each bound incarnation currently occupies (*Fireglass
    /// Mentor*, *You Find Some Prisoners*, *Queen Kayla bin-Kroog*, *Truth or
    /// Tale*). Appended to preserve replay identities.
    Found,
}

/// Optional zone movement applied to each card or permanent chosen (or left
/// unchosen) by [`super::Effect::ChooseCard`] or [`super::Effect::ChooseCardEx`].
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ChooseMove {
    /// Leave chosen object(s) in place; bind to `EventObject` and run `then`.
    None,
    /// Move each chosen object to its owner's hand.
    Hand,
    /// Put each chosen card onto the battlefield under the resolving controller's
    /// control, tapped when `tapped` is `true`.
    Battlefield {
        /// Whether the card enters tapped.
        tapped: bool,
    },
    /// Put each chosen object on top of its owner's library.
    TopOfLibrary,
    /// Put each chosen object on the bottom of its owner's library.
    BottomOfLibrary,
    /// Put each chosen object into its owner's graveyard.
    Graveyard,
    /// Exile each chosen object.
    Exile,
    /// Exile each chosen object and link it to the resolving ability's source
    /// (`Rider::Linked { host: source, until: None }`), so
    /// [`super::Effect::ReturnLinkedToBattlefield`] or
    /// [`super::Effect::ReturnLinkedToHand`] finds it when the source leaves
    /// (CR 607.2a). Appended to preserve replay identities.
    ExileLinked,
    /// Put the object(s) on the bottom of their owner's library in any order
    /// chosen by the resolving controller (`Pending::Arrange` when two or more
    /// cards go to one library). Appended to preserve replay identities.
    BottomOfLibraryChosenOrder,
    /// Put the object(s) on the bottom of their owner's library in a random
    /// order. Appended to preserve replay identities.
    BottomOfLibraryRandom,
}

/// Destination for cards chosen during [`super::Effect::PeekAndReveal`].
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum PeekPickDest {
    /// Leave chosen card(s) on top of the library (CR 701.20b).
    StayOnTop,
    /// Put chosen card(s) into the resolving controller's (or owner's) hand.
    Hand,
    /// Put chosen card(s) onto the battlefield under the resolving controller's
    /// control, tapped when `tapped` is `true`.
    Battlefield {
        /// Whether the card enters tapped.
        tapped: bool,
    },
    /// Put chosen card(s) on top of the library.
    TopOfLibrary,
    /// Put chosen card(s) into their owner's graveyard.
    Graveyard,
    /// Exile chosen card(s).
    Exile,
}

/// Destination for unchosen inspected cards in [`super::Effect::PeekAndReveal`].
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum PeekRestDest {
    /// Leave unchosen card(s) in their existing order on top of the library.
    StayInOrder,
    /// Put unchosen card(s) back on top of the library in any order chosen by
    /// the viewer.
    TopAnyOrder,
    /// Put unchosen card(s) on the bottom of the library (`random: true` for
    /// random order, `false` for viewer-chosen order).
    BottomOfLibrary {
        /// Whether the bottom order is randomized rather than chosen.
        random: bool,
    },
    /// Put unchosen card(s) into their owner's graveyard.
    Graveyard,
    /// Exile unchosen card(s).
    Exile,
}

/// Destination for matching card(s) found by [`super::Effect::RevealUntilThen`]
/// (`CR 701.20a`).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum RevealUntilFound {
    /// Leave matched card(s) in the revealed batch so [`RevealUntilRest`]
    /// places every revealed card together (*Treasure Hunt*, *Mutant's Prey*).
    SameAsRest,
    /// Leave matched card(s) in the library without moving them.
    StayInLibrary,
    /// Put each matched card into its owner's hand.
    Hand,
    /// Put each matched card onto the battlefield (`tapped` when `true`; under
    /// the resolving controller's control when `under_your_control` is `true`,
    /// or the library owner's control when `false`).
    Battlefield {
        /// Whether the card enters tapped.
        tapped: bool,
        /// `true` puts it under the resolving controller's control; `false`
        /// puts it under the library owner's control (*Oath of Druids*).
        under_your_control: bool,
    },
    /// Put each matched card on top of the library.
    TopOfLibrary,
    /// Put each matched card into its owner's graveyard.
    Graveyard,
    /// Exile each matched card.
    Exile,
    /// Exile each matched card and link it to the resolving ability's source
    /// (`Rider::Linked { host: source, until: None }`, CR 607.2a) so
    /// [`super::PlaySource::LinkedExile`] and linked-exile readers see it.
    ExileLinked,
}

/// Destination for non-matching card(s) turned over by
/// [`super::Effect::RevealUntilThen`] (and matched card(s) when
/// [`RevealUntilFound::SameAsRest`] is used).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum RevealUntilRest {
    /// Leave the cards in the library without moving them.
    StayInLibrary,
    /// Put the cards on top of the library.
    TopOfLibrary,
    /// Put the cards on the bottom of that library in a random order.
    BottomRandom,
    /// Put the cards on the bottom of that library in any order chosen by the
    /// resolving controller (`Pending::Arrange`).
    BottomChosenOrder,
    /// Return any exiled cards to that library and shuffle it.
    ShuffleIntoLibrary,
    /// Put the cards into their owner's hand.
    Hand,
    /// Put the cards into their owner's graveyard.
    Graveyard,
    /// Exile the cards.
    Exile,
}
