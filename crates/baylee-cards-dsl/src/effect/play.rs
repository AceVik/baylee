//! Windows and restrictions on playing a card exiled by an effect.
use baylee_core::types::TypeSet;

/// A permission follows the benefiting player's turns, including extra turns,
/// or lasts for as long as the exiled card or the granting source remains in its zone.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum PlayWindow {
    /// Expires in this turn's cleanup step.
    ThisTurn,
    /// Expires as the benefiting player's next turn starts.
    UntilYourNextTurn,
    /// Expires as the benefiting player's next actual upkeep begins.
    UntilYourNextUpkeep,
    /// Expires in the cleanup step of that player's next turn.
    ThroughYourNextTurn,
    /// Lasts for as long as the exiled card incarnation remains in exile.
    WhileExiled,
    /// Lasts for as long as the resolving source permanent's incarnation remains on the battlefield.
    WhileSourceOnBattlefield,
}

/// How mana may be spent to cast a spell under an exile permission (CR 609.4b, CR 118.14).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ExileMana {
    /// Ordinary mana spending rules apply.
    Exact,
    /// Spend mana as though it were mana of any color (`{C}` still requires colorless, CR 609.4b).
    AnyColor,
    /// Mana of any type can be spent to cast that spell (including `{C}`, CR 118.14).
    AnyType,
}

/// Composable play or cast permission granted on exiled card incarnations (CR 601.1a, CR 609.4).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ExilePermission {
    /// Duration of the permission.
    pub window: PlayWindow,
    /// `true` allows casting spells only (no land drops, CR 305.9); `false` allows playing lands and casting spells.
    pub cast_only: bool,
    /// Optional prospective spell-type filter (`None` permits any spell type).
    pub spell_types: Option<TypeSet>,
    /// Cast without paying its mana cost (CR 118.9).
    pub free: bool,
    /// Mana spending relaxation while casting this card (CR 609.4b).
    pub mana: ExileMana,
    /// Who receives the permission (`PlayerRel::You` by default;
    /// `PlayerRel::OwnerOfTarget` / `PlayerRel::OwnerOfEvent` grants the
    /// exiled card's owner permission, e.g. *Aerial Extortionist*).
    pub caster: super::PlayerRel,
}

impl ExilePermission {
    /// Grant permission to play (lands + spells) for `window` (CR 601.1a).
    #[must_use]
    pub const fn play(window: PlayWindow) -> Self {
        Self {
            window,
            cast_only: false,
            spell_types: None,
            free: false,
            mana: ExileMana::Exact,
            caster: super::PlayerRel::You,
        }
    }

    /// Grant permission to cast spells only (no land plays, CR 305.9) for `window`.
    #[must_use]
    pub const fn cast(window: PlayWindow) -> Self {
        Self {
            window,
            cast_only: true,
            spell_types: None,
            free: false,
            mana: ExileMana::Exact,
            caster: super::PlayerRel::You,
        }
    }

    /// Grant this permission to `caster` instead of only the resolving
    /// controller (for example `PlayerRel::OwnerOfTarget` for "its owner may
    /// cast it", *Aerial Extortionist*).
    #[must_use]
    pub const fn for_player(mut self, caster: super::PlayerRel) -> Self {
        self.caster = caster;
        self
    }

    /// Restrict casting to spells intersecting `spell_types` (implies `cast_only: true`).
    #[must_use]
    pub const fn with_spell_types(mut self, spell_types: TypeSet) -> Self {
        self.cast_only = true;
        self.spell_types = Some(spell_types);
        self
    }

    /// Allow casting without paying its mana cost (CR 118.9).
    #[must_use]
    pub const fn without_paying_mana_cost(mut self) -> Self {
        self.free = true;
        self
    }

    /// Allow spending mana as though it were mana of any color to cast it (CR 609.4b).
    #[must_use]
    pub const fn spend_mana_as_any_color(mut self) -> Self {
        self.mana = ExileMana::AnyColor;
        self
    }

    /// Allow spending mana of any type to cast it (CR 118.14).
    #[must_use]
    pub const fn spend_mana_as_any_type(mut self) -> Self {
        self.mana = ExileMana::AnyType;
        self
    }
}

/// How many library cards to exile and the permission granted for each.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ExilePlay {
    /// Top cards, or all remaining cards if the library contains fewer.
    pub count: u8,
    /// Lifetime independent of the source remaining on the battlefield.
    pub window: PlayWindow,
    /// Waive the mana cost, but still pay additional costs and increases.
    pub free: bool,
    /// Mana of any actual type may pay any mana symbol while casting this card.
    pub mana_any_type: bool,
    /// Restrict casting by the prospective spell's types; disallow land plays.
    /// `None` permits both casting spells and playing lands.
    pub spell_types: Option<TypeSet>,
}

/// One named option in an [`super::Effect::Vote`] ballot (`CR 701.38`).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct VoteOption {
    /// Option label shown to voters (for example `"time"`, `"knowledge"`).
    pub name: &'static str,
    /// Effects executed when this option is selected by the vote's [`VoteOutcome`].
    pub effects: &'static [super::Effect],
}

/// How an [`super::Effect::Vote`] resolves once all ballots are cast (`CR 701.38`).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum VoteOutcome {
    /// **Will of the council**: run the option with strictly the most votes, or
    /// `options[tied_option]` if tied for most votes.
    MajorityOrTied {
        /// Zero-based option index executed on a tie for most votes.
        tied_option: u8,
    },
    /// Run every option tied for the most votes in printed order (*Council Guardian*).
    AllWinners,
    /// **Council's dilemma**: record vote totals for [`super::Amount::VoteCount`]
    /// and execute each option's `effects` (whenever `votes[i] > 0`) in printed order.
    PerOptionCount,
    /// **Per-vote repetition** (*Expropriate*): for each option in printed order,
    /// execute its `effects` once per ballot cast for it with `EventPlayer` bound
    /// to that ballot's voter.
    RepeatPerVote,
}

/// What [`super::Effect::VoteObjects`] does to every object tied for the most votes.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum VoteObjectAction {
    /// Exile each winning object.
    Exile,
    /// Destroy each winning permanent (`no_regen` forbids regeneration).
    Destroy {
        /// Whether winning permanents can't be regenerated.
        no_regen: bool,
    },
    /// Return each winning object to its owner's hand.
    ReturnToHand,
}

/// Where [`super::Effect::Play`] gathers candidate cards from (`CR 608.2g`).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum PlaySource {
    /// Objects resolved through a [`super::TargetSpec`] (`FirstTarget`,
    /// `EventObject`, `Chosen`, `These`, etc.).
    Subject(super::TargetSpec),
    /// Cards exiled with the resolving source's current acquisition (`Rider::LinkedExile`).
    LinkedExile,
    /// Cards in `zone` (`Hand`, `Graveyard`, `Exile`, or `Library` top) owned by `owner`.
    Zone {
        /// Zone inspected for candidate cards.
        zone: crate::filter::ZoneRef,
        /// Player(s) whose zone is inspected.
        owner: super::PlayerRel,
    },
}

/// How many candidate cards [`super::Effect::Play`] offers to play/cast.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum PlayCount {
    /// Play/cast at most one matching card (`Amount$ 1`).
    One,
    /// Play/cast up to `n` matching cards.
    UpTo(u8),
    /// Play/cast any number of matching cards from the candidate set (`Amount$ All`).
    All,
}

/// Cost regime used by [`super::Effect::Play`] (`CR 118.9`, `CR 608.2g`).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum PlayCostMode {
    /// Cast without paying its mana cost (`CR 118.9`).
    WithoutPayingManaCost,
    /// Pay the spell's normal mana cost (`CR 608.2g`).
    Normal,
    /// Pay the spell's normal mana cost, spending mana as though it were mana of any type.
    AnyManaColor,
    /// Pay an explicit alternative mana cost rather than the card's mana cost (`PlayCost$`).
    Alternative(baylee_core::mana::ManaCost),
}
