//! Effect operations — the spell/ability effect vocabulary (v1).
//!
//! Operations are data; the engine interprets them. Anything not
//! expressible here is either an M2 primitive (continuous durations, copy,
//! phases) or a candidate for a flagged `// NOT SUPPORTED:` in the card.

use crate::KeywordSet;
use crate::cost::CostPart;
use crate::filter::Filter;
use baylee_core::color::ColorSet;
use baylee_core::ids::SubtypeId;
use baylee_core::mana::{ManaColor, ManaCost};
use baylee_core::types::{SupertypeSet, TypeSet};

/// Counter kinds (objects and players). Lives here so card definitions can
/// reference counters without engine dependencies.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, serde::Serialize, serde::Deserialize)]
pub enum CounterKind {
    /// A +X/+Y counter (CR 122.1a): it adds X to power and Y to toughness.
    ///
    /// One variant rather than a name per printed pair, because the rule is
    /// one rule and the pairs are open-ended — the reference corpus prints
    /// eleven of them (`P1P1`, `P1P0`, `P2P2`, `P0P1`, `P1P2`, and the
    /// mirrors), and a table of names would go silent on the twelfth.
    /// [`Self::P1P1`] is the const for the one Magic prints everywhere.
    Plus {
        /// X.
        power: u8,
        /// Y.
        toughness: u8,
    },
    /// A −X/−Y counter (CR 122.1a): it subtracts. The mirror of
    /// [`Self::Plus`], and a separate variant rather than one signed pair
    /// because the sign is part of what the counter *is*: CR 122.1a names
    /// exactly these two forms, CR 704.5q annihilates the +1/+1 against the
    /// −1/−1 and nothing else, and a card that counts its −0/−1 counters
    /// must not be answered with +0/−1 ones. Every P/T code the reference
    /// corpus prints is single-signed, which is the same fact measured from
    /// the other side.
    Minus {
        /// X.
        power: u8,
        /// Y.
        toughness: u8,
    },
    /// Loyalty.
    Loyalty,
    /// Lore (sagas).
    Lore,
    /// Time (suspend, vanishing).
    Time,
    /// Charge.
    Charge,
    /// Poison (players).
    Poison,
    /// Energy (players).
    Energy,
    /// Rad (players).
    Rad,
    /// Lifelink counter (grants lifelink, CR 122.1b).
    Lifelink,
    /// Level counters (classes, CR 716).
    Level,
    /// Card-specific counters.
    Custom(u16),
}

impl CounterKind {
    /// Parses a starting-position counter, including arbitrary signed P/T pairs.
    #[must_use]
    pub fn from_setup_name(name: &str) -> Option<Self> {
        match name.trim().to_ascii_lowercase().as_str() {
            "loyalty" => Some(Self::Loyalty),
            "lore" => Some(Self::Lore),
            "time" => Some(Self::Time),
            "charge" => Some(Self::Charge),
            "poison" => Some(Self::Poison),
            "energy" => Some(Self::Energy),
            "rad" => Some(Self::Rad),
            "lifelink" => Some(Self::Lifelink),
            "level" => Some(Self::Level),
            value => {
                if let Some(id) = value.strip_prefix("custom:") {
                    return id.parse().ok().map(Self::Custom);
                }
                let (a, b) = value.split_once('/')?;
                if a.starts_with('+') && b.starts_with('+') {
                    Some(Self::Plus {
                        power: a[1..].parse().ok()?,
                        toughness: b[1..].parse().ok()?,
                    })
                } else if a.starts_with('-') && b.starts_with('-') {
                    Some(Self::Minus {
                        power: a[1..].parse().ok()?,
                        toughness: b[1..].parse().ok()?,
                    })
                } else {
                    None
                }
            }
        }
    }

    /// The +1/+1 counter, which Magic prints on more cards than every other
    /// P/T pair together (2528 reference scripts against 195).
    ///
    /// A constant and not a variant, so that there is exactly one value for
    /// it: `Plus { power: 1, toughness: 1 }` and this name are the same
    /// value and compare equal, where a variant beside the general form
    /// would be two spellings nothing could keep in step.
    pub const P1P1: Self = Self::Plus {
        power: 1,
        toughness: 1,
    };
    /// The −1/−1 counter — [`Self::P1P1`]'s partner in CR 704.5q.
    pub const M1M1: Self = Self::Minus {
        power: 1,
        toughness: 1,
    };

    /// What this counter adds to power and toughness (CR 122.1a), or `None`
    /// for a counter that changes neither.
    ///
    /// The one place the arithmetic is written. Layer 7c (CR 613.4c) sums it
    /// over every counter a permanent wears, which is what lets a creature
    /// carry a −0/−1 and a +1/+1 at once without either being special-cased.
    #[must_use]
    pub const fn power_toughness(self) -> Option<(i16, i16)> {
        match self {
            Self::Plus { power, toughness } => Some((power as i16, toughness as i16)),
            Self::Minus { power, toughness } => Some((-(power as i16), -(toughness as i16))),
            _ => None,
        }
    }
}

/// Definition of a token a card can create.
///
/// A token is a permanent with no card behind it, which for a long time also
/// meant it could carry no rules: the engine reads abilities off the card in
/// the registry, and a token has none. `abilities` closes that hole — it is
/// the same [`crate::AbilityDef`] slice a card face carries, so a Treasure's
/// "{T}, Sacrifice this artifact: Add one mana of any color" is written and
/// executed exactly like the identical ability printed on a real card.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct TokenDef {
    /// Token name.
    pub name: &'static str,
    /// Colors.
    pub colors: ColorSet,
    /// Types.
    pub types: TypeSet,
    /// Supertypes.
    pub supertypes: SupertypeSet,
    /// Subtypes.
    pub subtypes: &'static [SubtypeId],
    /// Power (creatures).
    pub power: Option<i16>,
    /// Toughness (creatures).
    pub toughness: Option<i16>,
    /// Keywords.
    pub keywords: KeywordSet,
    /// Activated and triggered abilities, read exactly like a card face's.
    pub abilities: &'static [crate::ability::AbilityDef],
    /// A printed token card whose picture this token wears.
    ///
    /// The same third-party identifier [`crate::CardDef::scryfall_id`]
    /// carries, and here for the same reason: a token has no printing of its
    /// own in the game's print table, so without an id the client has nothing
    /// to draw and falls back to a flat coloured rectangle with the name
    /// written on it. Which is what a token looked like.
    ///
    /// It is rules data only in the sense that the rest of this struct is —
    /// the engine never reads it, and the picture is fetched at run time from
    /// a third party like every other card image (`docs/legal.md` §3). Empty
    /// means "no picture chosen", and a client draws the face instead rather
    /// than issuing a request that cannot succeed.
    pub scryfall_id: &'static str,
}

impl TokenDef {
    /// The blank token: no name, colorless, no types, no abilities.
    ///
    /// Every definition in `baylee_cards::tokens` is written as a
    /// struct-update tail on this, so adding a field does not mean editing
    /// every token — the same contract [`crate::CardDef::DEFAULT`] carries.
    pub const DEFAULT: Self = Self {
        name: "",
        colors: ColorSet::EMPTY,
        types: TypeSet::EMPTY,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[],
        power: None,
        toughness: None,
        keywords: KeywordSet::EMPTY,
        abilities: &[],
        scryfall_id: "",
    };
}

/// A computed number (CR 107.1).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Amount {
    /// A fixed value.
    Fixed(u32),
    /// The value of X chosen at cast time.
    X,
    /// The negated value of X (Toxic Deluge's `-X/-X`; evaluated as a
    /// negative at use sites).
    NegX,
    /// The value of X plus the controller's commander-cast count
    /// (Commander's Insight).
    XPlusCommanderCasts,
    /// Twice X (Heliod's Intervention).
    DoubleX,
    /// Number of distinct colors among battlefield objects matching the
    /// filter (General Tazri).
    DistinctColorsAmong(&'static Filter),
    /// Number of distinct **basic land types** among battlefield objects
    /// matching the filter — the count every "domain" card spells out.
    ///
    /// Domain is an ability word and so has no rules meaning of its own
    /// (CR 207.2c): what the card actually says is "for each basic land
    /// type among lands you control", which is why this takes a filter
    /// rather than being a bare `Domain` variant with "lands you control"
    /// hidden inside the engine.
    ///
    /// It is not [`Amount::CountOf`] and not [`Amount::DistinctColorsAmong`],
    /// which is what left three cards in the pool unwritable: `CountOf`
    /// counts *objects*, so one Tundra answers 1 where the card wants 2 and
    /// two Forests answer 2 where the card wants 1; and a land is
    /// colourless, so the colour count answers 0 for any of them. The five
    /// types are CR 305.6's, and no other land type counts.
    BasicLandTypesAmong(&'static Filter),
    /// A fixed negative value (-N at use sites).
    NegXFixed(u32),
    /// The power of the first target (last known characteristics).
    TargetPower,
    /// The power of the ability's own source (Esper Sentinel's tax).
    ///
    /// Read off the *projected* characteristics, so an anthem or a counter
    /// raises it — which is the whole reason the card prints `{X}` instead
    /// of `{1}`. A source that is not a creature, or that has left the
    /// battlefield, counts as zero rather than as its printed number: an
    /// ability whose amount comes off a permanent has nothing to read when
    /// the permanent is gone.
    SourcePower,
    /// How many counters of a kind are on the ability's own source — Aether
    /// Vial's "the number of charge counters on this artifact", cumulative
    /// upkeep's "for each age counter on it" (CR 702.24a).
    ///
    /// Read off the source as it is when asked, so after the effect before
    /// it in the same list put a new counter there; a source that has left
    /// the battlefield has shed its counters (CR 122.2) and counts zero,
    /// where CR 608.2h would read the last one it had. No card in the pool
    /// loses its source in between: Aether Vial taps itself and stays, and
    /// the intervening `if` cumulative upkeep prints keeps a gone source
    /// from mattering.
    CountersOnSource(CounterKind),
    /// The mana value of the first target (Reanimate's life loss).
    TargetCmc,
    /// "That much": the amount of damage the triggering event dealt
    /// (Questing Beast's redirect). Carried from the event onto the
    /// triggered ability as it goes on the stack; 0 anywhere else.
    EventAmount,
    /// "The sacrificed creature's mana value": the mana value, as it last
    /// existed on the battlefield (CR 608.2h), of the permanent sacrificed
    /// to pay the cost of the spell or ability that is resolving (Eldritch
    /// Evolution, Neoform, Birthing Pod).
    ///
    /// Read off the stack object, which is where the payment wrote it down:
    /// a spell's `Sacrifice` additional cost in the cast wizard, an
    /// activation's in `pay_cost`. Nothing sacrificed reads 0.
    SacrificedManaValue,
    /// "The amount of mana spent to cast this spell" (Memory Deluge): what
    /// the cast paid in mana (CR 601.2h), read off the stack object where
    /// the payment wrote it, as [`Self::SacrificedManaValue`] is. A free
    /// cast spent none; a flashback spent its flashback cost.
    ManaSpentToCast,
    /// "The tapped creature's power": the power of the permanent a
    /// `CostPart::TapOther` tapped to pay the cost of the ability that is
    /// resolving — station's "put a number of charge counters on this
    /// permanent equal to the tapped creature's power" (CR 702.184a). Its
    /// power as the effect applies while it is still on the battlefield as
    /// the same object, and as it last existed there otherwise (CR 608.2h).
    ///
    /// The creature is not a target (station targets nothing), so hexproof
    /// does not stop it. Read off the stack object, where `pay_cost` wrote
    /// which creature it tapped; nothing tapped reads 0.
    TappedPower,
    /// Number of objects matching a filter in a zone.
    CountOf {
        /// What to count.
        filter: &'static Filter,
        /// Where to count.
        zone: ZoneSel,
    },
    /// Another amount with a constant added to it: Muscle Burst's "3 plus
    /// the number of cards named Muscle Burst in all graveyards".
    ///
    /// A wrapper for the same reason [`Self::Negated`] is one — the offset
    /// is said once instead of doubling every counting amount there is — and
    /// it saturates rather than wraps, because a count is a count.
    ///
    /// The base is a **magnitude**: [`Self::is_negative`] reads a `Plus` as
    /// positive, because that is what the addition computes, and
    /// `amount_sign_tests::no_amount_in_the_pool_offsets_a_negative` is what
    /// keeps a negative base out rather than letting it resolve upwards.
    Plus {
        /// The amount being offset.
        base: &'static Amount,
        /// What is added to it.
        offset: u32,
    },
    /// The negation of another amount: "-1/-1 for each artifact you control"
    /// (Irradiate).
    ///
    /// [`Amount::NegX`] and [`Amount::NegXFixed`] are the two negatives the
    /// pool had before this, and they are the two whose magnitude is already
    /// a variant of its own. A *counted* quantity has no such twin — there is
    /// one `CountOf`, and a card wanting its negative had nothing to write —
    /// so this says the negation rather than doubling every amount there is.
    ///
    /// The sign is not in the evaluated number: the engine's `eval::amount`
    /// answers a magnitude, and every reader asks [`Amount::is_negative`] for
    /// the sign. Ask that function and never `matches!` on the variants: the
    /// three places that spelled the question out by hand would each have
    /// read this one as positive, which is a card that prints `-X/-X` and
    /// hands out `+X/+X`.
    Negated(&'static Amount),
}

impl Amount {
    /// Whether this amount counts **downwards**.
    ///
    /// The one place the question is answered, because it was four places
    /// before: `resolve::counters` spelled `matches!(a, Amount::NegX |
    /// Amount::NegXFixed(_))` in three separate closures and `baylee-ai`'s
    /// `tactics` had a fourth arm of its own. Four positive lists over an
    /// enum is four chances for the next negative amount to be read as a
    /// bonus, and nothing in a test suite reads a sign as a bug: the card
    /// resolves, the creature changes size, and only the direction is wrong.
    ///
    /// Nesting is answered by parity rather than refused, because that is
    /// what the word means. No card prints a double negative.
    #[must_use]
    pub const fn is_negative(&self) -> bool {
        match self {
            Self::NegX | Self::NegXFixed(_) => true,
            Self::Negated(inner) => !inner.is_negative(),
            _ => false,
        }
    }
}

/// Zone selectors for amounts/searches.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ZoneSel {
    /// The battlefield.
    Battlefield,
    /// Your library.
    LibraryYou,
    /// Your graveyard.
    GraveyardYou,
    /// All graveyards.
    GraveyardAll,
    /// Your hand.
    HandYou,
}

/// Which colors a mana effect may produce.
///
/// The colors, the amount and the spend restriction are three independent
/// questions, and they used to be answered by seven separate `Effect`
/// variants that each fixed all three — `AddMana`, `AddManaDynamic`,
/// `AddManaChoice`, `AddManaCommanderIdentity`,
/// `AddManaRestrictedCommanderIdentity`, `AddManaRestricted` and
/// `AddManaLandColor`. Anything the printed cards combined differently had
/// no way to be said.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ManaSource {
    /// One named color, or colorless.
    Fixed(ManaColor),
    /// A choice among the listed colors, made on resolution.
    Choice(&'static [ManaColor]),
    /// A color in your commander's color identity (Command Tower).
    CommanderIdentity,
    /// A color some land could produce: yours (Reflecting Pool) or an
    /// opponent's (Exotic Orchard).
    LandColor {
        /// `true` = your lands, `false` = opponents' lands.
        mine: bool,
        /// `true` for "any **type**" (Reflecting Pool), which colorless mana
        /// is (CR 106.1b); `false` for "any **color**" (Exotic Orchard,
        /// Fellwar Stone), which colorless is not (CR 106.1a) — so a Wastes
        /// across the table puts nothing on an Orchard's menu.
        any_type: bool,
    },
    /// The color chosen as this permanent entered (Uncharted Haven).
    ///
    /// Read off the *source* object rather than off the card, which is why
    /// `resolve::colors_of` takes the object at all: two Thriving Moors on
    /// one battlefield are two different colours, and a card cannot say
    /// which. A permanent with no chosen colour produces nothing, the same
    /// answer [`Self::LandColor`] gives when it finds no land.
    Chosen,
    /// `Add {W} or one mana of the chosen color.` — the chosen colour, or
    /// one of the colours printed beside it (the Thriving cycle).
    ///
    /// Its own variant rather than a flag on [`Self::Chosen`] for the reason
    /// `EnterModifier::ChooseColorExcept` is one: "Add one mana of the
    /// chosen color" names no alternative, and a card never restates a
    /// default.
    ChosenOr(&'static [ManaColor]),
}

/// What produced mana may be spent on (Cavern of Souls), or what spending
/// it on something does (Path of Ancestry), CR 106.6.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ManaRestriction {
    /// The spells the rider fires on, and while [`Self::restricts`] the only
    /// ones the mana may pay for.
    pub filter: &'static Filter,
    /// What happens when it is spent on a matching spell.
    pub rider: SpendRider,
    /// "Spend this mana only …". False for a rider alone ("When that mana
    /// is spent to cast …"): that mana is ordinary mana and pays for
    /// anything, and only a spell `filter` matches sets the rider off
    /// (#232).
    pub restricts: bool,
    /// This mana survives step and phase boundaries until end of turn.
    pub until_end_of_turn: bool,
}

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

/// Relative player references.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum PlayerRel {
    /// You (the controller).
    You,
    /// An opponent (M2: choice in multiplayer; auto-resolves heads-up).
    Opponent,
    /// Each player.
    EachPlayer,
    /// Each opponent.
    EachOpponent,
    /// The controller of the first target.
    ControllerOfTarget,
    /// The player who controlled the object the triggering event was about,
    /// as the event happened — Massacre Wurm's "whenever a creature an
    /// opponent controls dies, **that player** loses 2 life". Nothing is
    /// targeted (CR 115.1): the seat is read off the trigger's event, and
    /// last-known (CR 603.10a) because a creature that died is no longer
    /// controlled by anyone and a token that died no longer exists at all.
    ControllerOfEvent,
    /// The player chosen via `Pending::ChoosePlayer`.
    ///
    /// In `TargetSpec::CardInGraveyard` it is "from a single graveyard"
    /// (Unlicensed Hearse): the activation asks which graveyard before it
    /// asks for the targets, and offers only that one's cards.
    Chosen,
    /// "That player" of a trigger on damage dealt to a player — Ragavan,
    /// Nimble Pilferer's "whenever Ragavan deals combat damage to a player,
    /// … exile the top card of **that player's** library". Nothing is
    /// targeted (CR 115.1): the seat is read off the event the ability
    /// triggered on, and a player who has since left the game is nobody's
    /// "that player" (CR 800.4a).
    DamagedPlayer,
    /// The active player, the one whose turn it is (CR 102.1): "that
    /// player" of a trigger at the beginning of a step — Copper Tablet's
    /// "at the beginning of each player's upkeep, this artifact deals 1
    /// damage to that player". The ability resolves in the step it
    /// triggered in, so the player whose step it was is still the active
    /// one.
    ActivePlayer,
    /// The controller of the permanent the source is attached to —
    /// "enchanted land's controller" (Cursed Land). Not the Aura's own
    /// controller: the two need not be the same (CR 303.4e).
    ControllerOfAttached,
}

/// Target specifications (chosen at cast/activation, CR 601.2c).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum TargetSpec {
    /// An object matching the filter (battlefield, or stack for spells).
    Object(&'static Filter),
    /// "For each opponent, … up to one target [filter] that player
    /// controls" (The True Scriptures I): one question per opponent, each
    /// offering only that player's permanents, the answers gathered into
    /// one list. Written with `TargetReq::up_to(spec, u8::MAX)`; the count
    /// is the opponents' (CR 601.2c, 115.1).
    ObjectOfEachOpponent(&'static Filter),
    /// "Target opponent or [filter]" (Ravager of the Fells: "target opponent
    /// or planeswalker"): one choice over the opponents and the permanents
    /// the filter matches, offered together the way "any target" offers its
    /// two lists (CR 115.1d).
    OpponentOrObject(&'static Filter),
    /// "Target [filter] that player or that planeswalker's controller
    /// controls" — a **second** instance of "target" whose permanents are
    /// those of the player the first instance named, or of the controller of
    /// the permanent it named (Ravager of the Fells). Written only as an
    /// ability's `second_targets`; the engine binds it to
    /// [`Self::ObjectControlledBy`] once the first answer is in, since the
    /// targets of one instance are chosen before the next (CR 601.2c).
    ObjectOfFirstTargetsPlayer(&'static Filter),
    /// A permanent matching the filter that one named player controls. Never
    /// written on a card: it is what a spec that names "that player" becomes
    /// once the engine knows which player that is, kept on the stack object
    /// so the resolution-time re-check (CR 608.2b) asks the same question
    /// the offer did.
    ObjectControlledBy(&'static Filter, baylee_core::ids::PlayerId),
    /// "Target [filter] that player controls", where "that player" is the
    /// one the triggering event dealt damage to (Questing Beast: "it deals
    /// that much damage to target planeswalker that player controls"). The
    /// engine binds it to [`Self::ObjectControlledBy`] as the trigger is
    /// put on the stack.
    ObjectOfEventPlayer(&'static Filter),
    /// A spell on the stack matching the filter.
    Spell(&'static Filter),
    /// A spell on the stack OR a permanent on the battlefield (Venser).
    StackOrBattlefield(&'static Filter),
    /// A card in a graveyard matching the filter.
    CardInGraveyard(&'static Filter, PlayerRel),
    /// A graveyard card below the triggering permanent's last battlefield
    /// mana value. The engine binds this before offering targets.
    CardInGraveyardBelowEvent(&'static Filter, PlayerRel),
    /// A captured strict mana-value bound, retained on a stacked trigger
    /// for target rechecks and copies. Card definitions use `BelowEvent`.
    CardInGraveyardBelowValue(&'static Filter, PlayerRel, u32),
    /// The source object.
    ThisObject,
    /// An activated/triggered ability on the stack (Tishana's Tidebinder).
    AbilityOnStack(&'static Filter),
    /// A spell or ability on the stack (Ertai Resurrected's counter mode).
    SpellOrAbility(&'static Filter),
    /// The object the triggering event was about (Wartime Protestors'
    /// "that creature").
    EventObject,
    /// The players a [`PlayerRel`] names, relative to the controller — every
    /// one of them, and no choice. `Player(EachOpponent)` is "each opponent",
    /// `Player(EachPlayer)` "each player", `Player(You)` "you": Mount Doom's
    /// and Ramunap Ruins' "deals 1 damage to each opponent" is
    /// `DealDamage { target: Player(EachOpponent) }`. With no `targets = …`
    /// on the ability it is not a target (CR 115.10a); a printed "target
    /// opponent" is [`Self::AnyOpponent`].
    Player(PlayerRel),
    /// Any player (choice via `Pending::ChoosePlayer`).
    AnyPlayer,
    /// Any *opponent* — the same choice as [`Self::AnyPlayer`] over a
    /// smaller set.
    ///
    /// Its own variant rather than `AnyPlayer` with a filter, because a
    /// player has no characteristics to filter on, and rather than
    /// `Player(PlayerRel::Opponent)`, which is *every* opponent and no
    /// choice at all. "Target opponent" printed on a card is one opponent,
    /// picked, and in a game of four that is three different things.
    AnyOpponent,
    /// "Any target" (CR 115.4): a creature, a planeswalker, a battle, **or
    /// a player** — one choice over a set that spans objects and players.
    ///
    /// It is its own variant rather than a `Filter`, because no filter can
    /// match a player: a player has no characteristics to filter on. Every
    /// burn spell printed says this, so a transcoder that reads it as
    /// "matches everything on the battlefield" produces cards that cannot
    /// point at a player and still call themselves implemented.
    AnyTarget,
}

/// How many targets an ability/spell requires.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct TargetReq {
    /// What may be targeted.
    pub spec: TargetSpec,
    /// Minimum number of targets (0 = may decline).
    pub min: u8,
    /// Maximum number of targets (255 = "any number", X-driven).
    pub max: u8,
    /// Whether the count is exactly X (Curse of the Swine).
    pub count_is_x: bool,
}

impl TargetReq {
    /// Exactly one target.
    pub const fn one(spec: TargetSpec) -> Self {
        Self {
            spec,
            min: 1,
            max: 1,
            count_is_x: false,
        }
    }

    /// Exactly `n` targets ("tap two target lands").
    pub const fn exactly(spec: TargetSpec, n: u8) -> Self {
        Self {
            spec,
            min: n,
            max: n,
            count_is_x: false,
        }
    }

    /// How many targets may be chosen, once `x` has been announced.
    ///
    /// "X target creatures" is exactly X, and every other requirement is its
    /// own bounds. This is the one reading of "how many" that both the cast
    /// wizard and an activation use. X above 255 is capped there, since no
    /// board holds more objects than that.
    #[must_use]
    pub const fn bounds(self, x: u32) -> (u8, u8) {
        if self.count_is_x {
            let n = if x > 255 { 255 } else { x as u8 };
            (n, n)
        } else {
            (self.min, self.max)
        }
    }

    /// Up to one target.
    pub const fn up_to_one(spec: TargetSpec) -> Self {
        Self {
            spec,
            min: 0,
            max: 1,
            count_is_x: false,
        }
    }

    /// Up to `max` targets.
    pub const fn up_to(spec: TargetSpec, max: u8) -> Self {
        Self {
            spec,
            min: 0,
            max,
            count_is_x: false,
        }
    }

    /// Exactly X targets.
    pub const fn x_targets(spec: TargetSpec) -> Self {
        Self {
            spec,
            min: 0,
            max: 255,
            count_is_x: true,
        }
    }
}

/// Which object an effect names when an ability says "target" twice.
///
/// Every other effect reads **one** instance of the word: `res.targets` is the
/// list chosen for the ability's [`TargetReq`], and an effect that points at
/// "the target" points at that list. A fight is the sentence that cannot be
/// said that way — "target creature you control fights target creature you
/// don't control" is two instances, each with its own requirement and its own
/// answer (CR 115.3 lets one object be chosen for each), and the effect is a
/// relation *between* them. So the two sides of it are named here rather than
/// by adding positions to the one list: a second target appended to
/// `targets` would change what every existing reader of it means.
///
/// [`TargetSlot::This`] is the third participant a fight can have — "this
/// creature fights target creature" (CR 701.14a's first half) — and is what
/// lets that shape be written with **one** requirement.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum TargetSlot {
    /// The source of the spell or ability.
    This,
    /// The object chosen for the ability's first instance of "target".
    First,
    /// The object chosen for its second instance
    /// (`second_targets` on the ability).
    Second,
}

/// What happens when mana is spent on a spell its filter matches.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum SpendRider {
    /// Nothing extra (restriction only).
    None,
    /// The spell can't be countered (Cavern of Souls).
    Uncounterable,
    /// The caster scries N (Path of Ancestry).
    Scry(u8),
}

/// How a card's mana value is compared with a [`ManaValueBound`]'s amount.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ManaValueCmp {
    /// "With mana value X or less."
    AtMost,
    /// "With mana value equal to …."
    Exactly,
}

/// "A creature card with mana value equal to 1 plus the sacrificed
/// creature's mana value": a bound whose number the resolution computes.
///
/// A `Filter` cannot carry it — a filter is asked with no resolution in
/// hand, and the sacrificed creature is written on the stack object that is
/// resolving, not on the source a filter can see.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ManaValueBound {
    /// How the card's mana value is compared.
    pub cmp: ManaValueCmp,
    /// What it is compared with.
    pub amount: Amount,
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

/// A single effect operation.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Effect {
    /// Run operations in order.
    Sequence(&'static [Effect]),
    /// You gain life.
    GainLife {
        /// How much.
        amount: Amount,
    },
    /// A chosen relative player gains life.
    GainLifeFor {
        /// How much.
        amount: Amount,
        /// Who.
        who: PlayerRel,
    },
    /// Exile a target object.
    Exile {
        /// What.
        target: TargetSpec,
    },
    /// "It deals `amount` damage divided as you choose among any number of
    /// target creatures and/or planeswalkers" (Fury): the ability's targets
    /// share the damage as its controller divided it.
    ///
    /// The division is announced as the ability is put on the stack, after
    /// its targets (CR 601.2d, which CR 603.3d applies to a triggered
    /// ability), and every target is given at least 1 — so the ability's
    /// `TargetReq` asks for at most `amount` of them. It is asked target by
    /// target in the order they were chosen; the last one takes the rest.
    /// At resolution a target that has become illegal is dealt nothing, and
    /// its share goes to nobody else (CR 608.2b).
    ///
    /// A fixed amount, and only on a triggered ability: nothing in the cast
    /// wizard asks a division yet, so Fire // Ice's "2 damage divided as you
    /// choose among one or two targets" and Shatterskull Smashing's X are
    /// not said by this. `lints::every_divided_damage_is_a_trigger_that_can_divide`
    /// holds both halves.
    DealDamageDivided {
        /// The damage divided.
        amount: u32,
    },
    /// Exile a target and return it to the battlefield immediately
    /// (Ephemerate, Restoration Angel). Written with [`Effect::blink_to_owner`]
    /// or [`Effect::blink_to_you`], the two sentences the pool prints.
    Blink {
        /// What.
        target: TargetSpec,
        /// Under whose control the card comes back: its owner's (`true`) or
        /// that of the player who controls the resolving spell or ability
        /// (`false`).
        ///
        /// The same field, and the same question, as
        /// [`Effect::GraveyardToBattlefield`]'s. The card that returns is a
        /// new object (CR 400.7), so whatever control effect held the one
        /// that was exiled is gone with it, and the new one enters under the
        /// control the sentence names. That is a printed choice, not a rule:
        /// CR 610.3c's "under its owner's control unless otherwise
        /// specified" is about a card returned by a *second* one-shot effect
        /// after an "until" event, which an immediate blink is not. So
        /// Ephemerate, Soulherder and Emiel the Blessed print "under its
        /// owner's control" and take `true`, while Restoration Angel,
        /// Aminatou's −1 and Sword of Hearth and Home print "under your
        /// control" and take `false` — and a player notices the difference
        /// the moment the creature they flicker is one they stole.
        ///
        /// Only control is chosen here. The owner never changes (CR 108.3),
        /// so a stolen creature kept this way still dies into its owner's
        /// graveyard (CR 400.3) and still leaves the game with its owner
        /// (CR 800.4a).
        owner_control: bool,
    },
    /// Look at the top `count` cards of your library; put `pick` of them
    /// into your hand and the rest on the bottom — in any order (Dig
    /// Through Time), or in a random order when `random` (Memory Deluge,
    /// Consult the Star Charts). `count` is read as the effect begins.
    LookAtTopPick {
        /// How many to look at.
        count: Amount,
        /// How many to keep.
        pick: u8,
        /// "In a random order": the rest are shuffled onto the bottom and
        /// nobody is asked.
        random: bool,
    },
    /// Look at the top `count` cards of your library; put one of them into
    /// your hand, one on the bottom of your library, and exile the rest —
    /// which you may play this turn (Expressive Iteration). Two choices in
    /// that order: the card for the hand, then the card for the bottom.
    LookAtTopKeepBottomPlay {
        /// How many to look at.
        count: u8,
    },
    /// "Choose an exiled card an opponent owns with a void counter on it.
    /// You may play it this turn without paying its mana cost." (Dauthi
    /// Voidwalker) — the choice is of a card in any exile whose owner
    /// stands in `owner` to you and which carries `counter`, and the answer
    /// is a permission for you to play that object this turn
    /// (`PlayPermission` in the engine), free when `free`. Not a target:
    /// "choose" (CR 115.10a says only the word "target" makes one).
    ChooseExiledToPlay {
        /// Whose cards: the owner's relation to you.
        owner: PlayerRel,
        /// A counter the card must carry.
        counter: Option<CounterKind>,
        /// "Without paying its mana cost".
        free: bool,
    },
    /// "Reveal the top `count` cards of your library. An opponent separates
    /// those cards into two piles. Put one pile into your hand and the other
    /// into your graveyard." (Fact or Fiction.) The opponent answers a
    /// `ChooseCards` naming the first pile (`ChoicePrompt::FirstPile`, any
    /// number, the rest are the second), and the controller a
    /// `Pending::ChoosePile`. At a table with several opponents the
    /// controller first names the one who separates.
    RevealAndSeparate {
        /// Cards revealed.
        count: u32,
    },
    /// "Mill `amount` cards. You may put a [filter] card from among the
    /// milled cards into your hand." (Wrenn and Realmbreaker's −2.) The
    /// choice is a `ChooseCards` with `min: 0` over the milled cards that
    /// match, found wherever they went if that zone is public (CR 701.17c),
    /// a replacement's exile included, and none matching asks nothing.
    MillMayTakeOne {
        /// Cards milled.
        amount: u32,
        /// What may be taken.
        filter: &'static Filter,
    },
    /// Cascade's effect (CR 702.85a): "exile cards from the top of your
    /// library until you exile a nonland card whose mana value is less than
    /// this spell's mana value. You may cast that card without paying its
    /// mana cost if the resulting spell's mana value is less than this
    /// spell's mana value. Then put all cards exiled this way that weren't
    /// cast on the bottom of your library in a random order."
    ///
    /// The body of a `Trigger::SpellCast(&Filter::This)` ability — "when you
    /// cast this spell" — so "this spell" is the ability's source. The cast
    /// is asked as a `YesNoPrompt::CastWithoutPaying` and made the moment
    /// the ability has finished resolving, through the free-cast wizard, so
    /// targets and modes are chosen as for any spell; a card that turns out
    /// not to be castable goes to the bottom with the rest. "Cascade,
    /// cascade" is two of these abilities (CR 702.85c).
    Cascade,
    /// "You may cast that card", where the card is the ability's first
    /// target (Conduit of Worlds), as the ability resolves (CR 608.2g),
    /// paying its costs; the timing its type would impose does not apply,
    /// because nobody is casting it with priority.
    ///
    /// Asked as a `YesNoPrompt::CastPaying`. A yes opens a CR 605.3a
    /// payment window for the card's mana cost the moment the ability has
    /// finished resolving; passing it casts the card through the cast
    /// wizard, paid out of the pool, and a pool that cannot pay casts
    /// nothing. `then_no_more_spells` is "If you do, you can't cast
    /// additional spells this turn", set once the spell has been cast.
    MayCastTarget {
        /// Whether casting it forbids further spells this turn.
        then_no_more_spells: bool,
    },
    /// "Reveal cards from the top of your library until you reveal a
    /// [filter] card. Put that card [where `found` says] and the rest on the
    /// bottom of your library in a random order." (Nissa, Resurgent
    /// Animist.) No player is asked anything: every revealed card is shown
    /// to every player (CR 701.20a), the first match goes where `found`
    /// says, and the rest go to the bottom in an order the table's generator
    /// picks. A library with no match reveals every card and puts them all
    /// on the bottom, again at random.
    RevealUntil {
        /// What stops the reveal.
        filter: &'static Filter,
        /// Where the match goes.
        found: SearchDest,
    },
    /// Reveal the top card of your library and put it where the filter
    /// sends it: `matched` if it is a `filter` card, `otherwise` if not
    /// (Coiling Oracle: "If it's a land card, put it onto the battlefield.
    /// Otherwise, put that card into your hand.").
    ///
    /// No choice anywhere in it, so no player is asked: the card is shown
    /// to every player (CR 701.20a) and then goes where the text says. An
    /// empty library reveals nothing and does nothing.
    RevealTopAndSort {
        /// What the revealed card is asked about.
        filter: &'static Filter,
        /// Where it goes if it matches.
        matched: SearchDest,
        /// Where it goes if it does not.
        otherwise: SearchDest,
    },
    /// Look at the top card of your library; if it is a `filter` card you
    /// may put it where `matched` says, and if you don't — or it is not —
    /// put it `otherwise` (Risen Reef: "If it's a land card, you may put it
    /// onto the battlefield tapped. If you don't put the card onto the
    /// battlefield, put it into your hand.").
    ///
    /// [`Self::RevealTopAndSort`]'s sibling with the two words that make it
    /// a different sentence: "look", so nothing is shown to the table, and
    /// "you may", so a matching card is a question — asked as a choice of
    /// that one card, which is what lets the asked player see it. An empty
    /// library does nothing.
    LookAtTopMayPut {
        /// What the card has to be for the question to be asked.
        filter: &'static Filter,
        /// Where a matching card goes if the player puts it there, and
        /// whether it enters tapped.
        matched: Find,
        /// Where the card goes otherwise.
        otherwise: SearchDest,
    },
    /// "Search your library and/or graveyard for a [filter] card and put it
    /// [where `find` says]. If you search your library this way, shuffle."
    /// (Finale of Devastation.) One card from either zone: the graveyard is
    /// public, so its matches are offered first (`ChoicePrompt::FromGraveyard`,
    /// naming none to search the library instead); a graveyard card taken is
    /// the whole search and nothing is shuffled. Declining, or a graveyard
    /// with no match, is the library search `SearchLibrary` makes, shuffle
    /// and all — so a library once seen is always shuffled.
    SearchLibraryOrGraveyard {
        /// What may be found.
        filter: &'static Filter,
        /// Where the card goes.
        find: &'static Find,
    },
    /// "You may discard up to `count` cards. If you do, draw that many
    /// cards." (Fable of the Mirror-Breaker's chapter II.) One question —
    /// which cards, naming none to decline — and the draw is the number of
    /// cards actually discarded, read off the answer rather than printed.
    DiscardUpToThenDraw {
        /// The most that may be discarded.
        count: u8,
    },
    /// Put cards from your hand on top of your library, in the order they
    /// were chosen (Brainstorm-style).
    PutFromHandOnTop {
        /// How many.
        count: u8,
    },
    /// "Choose `count` cards in your hand drawn this turn. For each of those
    /// cards, pay `life` life or put the card on top of your library"
    /// (Sylvan Library). Two questions: which cards — asked only when more
    /// than `count` were drawn and are still in the hand — and then which of
    /// them go back on top, in the order put back; the rest are paid for.
    /// A card whose life cannot be paid has to go back (CR 119.4), which is
    /// the second question's minimum.
    PayLifeOrPutBackDrawn {
        /// How many drawn cards are chosen.
        count: u8,
        /// The life each one kept costs.
        life: u16,
    },
    /// You put a card matching the filter from your hand onto the
    /// battlefield, untapped and under your control — Aether Vial's "you may
    /// put a creature card with mana value equal to the number of charge
    /// counters on this artifact from your hand onto the battlefield", Uro's
    /// "you may put a land card from your hand onto the battlefield".
    ///
    /// Not a cast and not a land play (CR 305.4): nothing is paid, and no
    /// land drop is spent. `mana_value` is read as the effect begins, as
    /// [`Effect::SearchLibraryOf`]'s is.
    PutFromHandOntoBattlefield {
        /// What may be put.
        filter: &'static Filter,
        /// A mana-value bound read at resolution, on top of `filter`.
        mana_value: Option<ManaValueBound>,
        /// "You may" — the chooser may put nothing.
        optional: bool,
    },
    /// A player loses life.
    LoseLife {
        /// How much.
        amount: Amount,
        /// Who.
        target: PlayerRel,
    },
    /// You draw cards.
    DrawCards {
        /// How many.
        amount: Amount,
    },
    /// A relative player draws cards.
    DrawCardsFor {
        /// How many.
        amount: Amount,
        /// Who.
        who: PlayerRel,
    },
    /// Exile all targets; each exiled permanent's controller creates the
    /// token (Curse of the Swine).
    ExileTargetsCreateTokens {
        /// The token to create per exiled permanent.
        token: &'static TokenDef,
    },
    /// Deal damage to a target.
    DealDamage {
        /// How much.
        amount: Amount,
        /// To what.
        target: TargetSpec,
    },
    /// Two creatures fight (CR 701.14a): each deals damage equal to its power
    /// to the other, at once, and none of it is combat damage (CR 701.14d).
    ///
    /// CR 701.14b is the half that is easy to get backwards: if either
    /// creature has left the battlefield, stopped being a creature, or is an
    /// illegal target as this resolves, **neither** deals damage — the one
    /// that is still there does not hit the other on its own. A creature
    /// fighting itself deals twice its power to itself (CR 701.14c), which
    /// the two halves say without a special case.
    Fight {
        /// The creature told to fight.
        fighter: TargetSlot,
        /// The creature it fights.
        foe: TargetSlot,
    },
    /// "`dealer` deals damage equal to its power to `to`" — the one-sided
    /// half of a fight, which is not a fight: nothing is dealt back.
    ///
    /// Held to the same "both or nothing" as [`Effect::Fight`], but by
    /// CR 608.2b rather than CR 701.14b: an illegal dealer is one whose power
    /// the effect "fails to determine", so no damage happens, and an illegal
    /// recipient is not affected by the part of the effect it is illegal for.
    DamageEqualToPower {
        /// The creature whose power is the amount, and which deals it.
        dealer: TargetSlot,
        /// What is dealt to — a creature, or a planeswalker (CR 306.8).
        to: TargetSlot,
    },
    /// "that creature deals damage equal to its power to `target`", where
    /// "that creature" is the object the trigger's event named (Pyrogoyf:
    /// "Whenever this creature or another Lhurgoyf creature you control
    /// enters, that creature deals damage equal to its power to any
    /// target").
    ///
    /// The creature is the damage's source (CR 120.3 reads the source's
    /// deathtouch and lifelink), and if it has left the battlefield by the
    /// time the ability resolves, its power and its characteristics are as
    /// it last existed there (CR 608.2h).
    EventObjectDealsDamageEqualToPower {
        /// What is dealt to.
        target: TargetSpec,
    },
    /// Deal damage to the first target's controller (Tuktuk Scrapper).
    DealDamageToTargetController {
        /// How much.
        amount: Amount,
    },
    /// "~ deals N damage to each [filter]" — every permanent the filter
    /// matches as this resolves, all at once (CR 608.2f), the set read once
    /// (CR 608.2h), and none of them a target (CR 115.10a): hexproof and
    /// shroud do not stop it, nothing is chosen, and it asks nobody anything.
    ///
    /// Only a creature or a planeswalker is dealt it, whatever the filter
    /// says, for two different reasons. A matched land or artifact is skipped
    /// because damage can't be dealt to one (CR 120.1a). A matched battle is
    /// skipped because of an engine gap, not a rule: CR 120.3h removes
    /// defense counters, and `CounterKind` has none. The skip is a backstop
    /// and not the spelling — the filter names the printed noun, so "each
    /// creature with flying" is `And(&[CREATURE, HasKeyword(FLYING)])` and
    /// never `HasKeyword(FLYING)` alone. A planeswalker loses loyalty instead
    /// of being marked (CR 120.3c).
    ///
    /// Objects only, on purpose. "…and each player", "to you and each
    /// creature you control" and "to each opponent" are
    /// `DealDamage { target: TargetSpec::Player(rel) }` in the same list, so
    /// every sentence has one spelling rather than two.
    DealDamageEach {
        /// How much, read once before anything is dealt.
        amount: Amount,
        /// Which permanents.
        filter: &'static Filter,
    },
    /// "Prevent the next N damage that would be dealt to any target this
    /// turn" (Samite Healer; CR 615.7): a shield on each recipient the
    /// target names, reduced by 1 for each 1 damage it prevents and gone
    /// once it reaches 0 or the turn's cleanup ends it (CR 514.2).
    ///
    /// The recipient is fixed as this resolves and the shield is on that
    /// object — a creature that leaves the battlefield and comes back is a
    /// new object (CR 400.7) with no shield.
    PreventNextDamage {
        /// Whom the shield is on, as [`Effect::DealDamage`] names a
        /// recipient.
        target: TargetSpec,
        /// How much it prevents in all.
        amount: Amount,
    },
    /// "Prevent all combat damage that would be dealt this turn" (Fog):
    /// every combat damage event until the turn's cleanup, to anything
    /// and from anything, and never used up.
    PreventAllCombatDamageThisTurn,
    /// "You may reveal a card you own from outside the game, or choose a
    /// face-up card you own in exile. Put that card into your hand."
    /// (wishes; Karn, the Great Creator's −2).
    WishToHand {
        /// Which cards qualify.
        filter: &'static Filter,
    },
    /// Destroy a target permanent (CR 701.8a).
    Destroy {
        /// What.
        target: TargetSpec,
        /// Whether the printing adds "it can't be regenerated"
        /// (CR 701.19c): a regeneration shield on the permanent is *not*
        /// applied, and the destruction goes through.
        ///
        /// A field rather than two variants because it is a rider on one
        /// sentence — eight cards in this pool print it and a hundred and
        /// twenty-two do not, with no card on both doors — and
        /// [`Effect::destroy`] supplies the `false`
        /// so no card restates the default. The doc above this used to say
        /// every destroy was unregeneratable, which was true only for as
        /// long as no shield existed.
        no_regen: bool,
    },
    /// Put each target on the bottom of its owner's library (Banishing
    /// Stroke).
    PutTargetOnBottomOfLibrary,
    /// Put a card that is in a graveyard on the bottom of its owner's
    /// library: Murderous Rider's "when this creature dies, put it on the
    /// bottom of its owner's library", where "it" is
    /// [`TargetSpec::EventObject`] and nothing is targeted.
    ///
    /// Its own variant rather than [`Self::PutTargetOnBottomOfLibrary`]
    /// aimed at the card that died, because that one moves its target from
    /// wherever it is: a card that left the graveyard in response is a new
    /// object (CR 400.7) the trigger knows nothing about, and it stays where
    /// it went (#240).
    PutOnBottomOfLibraryFromGraveyard {
        /// Which card, read at resolution.
        target: TargetSpec,
    },
    /// The first target (a card in a graveyard) gains flashback with
    /// flashback cost = its mana cost until end of turn (Snapcaster
    /// Mage).
    GrantFlashback,
    /// The controller takes an extra turn after this one (Temporal
    /// Mastery).
    TakeExtraTurn,
    /// Exile the source object (Temporal Mastery's self-exile rider).
    ExileSource,
    /// Tap each target.
    TapTarget,
    /// Untap each target.
    UntapTarget,
    /// Untap the source permanent, which names no target and asks nobody
    /// anything (Basalt Monolith's `{3}: Untap this artifact`).
    ///
    /// Not [`Self::UntapTarget`] with a self-filter. "Untap this artifact"
    /// is not a targeted ability — CR 115.1c makes an activated ability
    /// targeted only when it says the phrase "target [something]" — so it
    /// cannot be made illegal by hexproof or shroud, and — the difference a
    /// player sees — is not asked about: a `TargetSpec` puts a
    /// `Pending::ChooseTargets` with exactly one legal answer in front of
    /// somebody.
    ///
    /// Three cards in this pool print it: Devoted Druid, which plays, and
    /// Basalt Monolith and Grim Monolith, which also print "doesn't untap
    /// during your untap step" and wait on G5. (Frantic Search, Restless
    /// Ridgeline and Song-Mad Treachery untap *something else* without
    /// targeting it, which is a different effect and not this one.)
    UntapSelf,
    /// Exile each target; return it to the battlefield under its owner's
    /// control at the beginning of the next end step (Venser +2).
    ExileAndReturnAtEndStep,
    /// "Its owner puts it on their choice of the top or bottom of their
    /// library" (Subtlety). The target leaves the stack or the battlefield
    /// for its owner's library, and the **owner** picks the end, whoever
    /// controls this ability. Not a counter: a spell that can't be countered
    /// goes all the same.
    OwnerPutsOnTopOrBottom {
        /// What (a spell or a permanent the ability targeted).
        target: TargetSpec,
    },
    /// "If that creature would die this turn, exile it instead" (Mawloc):
    /// a replacement effect (CR 614.1a) on each object the spec names, for
    /// the rest of the turn and for that object only — a creature that left
    /// the battlefield and came back is a new object (CR 400.7) and dies as
    /// usual. A token is exiled instead as well, and does not die.
    ExileIfDiesThisTurn {
        /// Which creature.
        target: TargetSpec,
    },
    /// Discover N (CR 701.57a): "Exile cards from the top of your library
    /// until you exile a nonland card with mana value N or less. You may
    /// cast that card without paying its mana cost if the resulting spell's
    /// mana value is less than or equal to N. If you don't cast it, put
    /// that card into your hand. Put the remaining exiled cards on the
    /// bottom of your library in a random order." (Trumpeting Carnosaur.)
    ///
    /// The exiling and the random bottom happen as the ability resolves;
    /// the cast is offered as soon as the resolution is over, before anybody
    /// receives priority (the engine's `GameState::discovered`), and a card
    /// that cannot be cast, or that its owner declines, goes to the hand.
    Discover {
        /// N: the highest mana value that stops the exiling.
        mana_value: u8,
    },
    /// "Reveal the top `count` cards of your library. For each card type,
    /// you may put a card of that type from among the revealed cards into
    /// your hand. Put the rest on the bottom of your library in a random
    /// order." (Atraxa, Grand Unifier.)
    ///
    /// One question per card type (CR 205.2a) that a revealed card still in
    /// the library has, in that rule's order: up to one card of that type.
    /// A card taken for one type is out of the later questions, which is
    /// what "a card of that type" for each type means for a card with two:
    /// it is put into the hand once, for one of them, and every set of
    /// cards the sentence allows is some sequence of answers.
    RevealTopOnePerType {
        /// How many cards are revealed.
        count: u8,
    },
    /// Counter a spell on the stack; it goes to exile instead of the
    /// graveyard (Force of Negation).
    CounterTargetSpellToExile,
    /// Counter a spell on the stack.
    CounterTargetSpell,
    /// Counter an activated or triggered ability on the stack (Tishana's
    /// Tidebinder).
    CounterTargetAbility,
    /// Counter the first target regardless of whether it is a spell or an
    /// ability on the stack (Ertai Resurrected).
    CounterTargetSpellOrAbility,
    /// The permanent whose ability an earlier [`Effect::CounterTargetAbility`]
    /// of the same resolution countered loses its abilities, for as long as
    /// the source of this effect remains on the battlefield (Tishana's
    /// Tidebinder).
    ///
    /// `source_filter` is the printed restriction — the Tidebinder's rider
    /// reaches "an artifact, creature, or planeswalker" and nothing else —
    /// and it is data on the card rather than a rule in the engine because
    /// the next card to say this will draw the line somewhere else.
    TargetSourceLosesAbilities {
        /// Which permanents the rider reaches.
        source_filter: &'static Filter,
    },
    /// Register delayed mana at the controller's next first main phase
    /// (Mana Drain): colorless mana equal to the first target's cmc.
    DelayedManaAtNextFirstMain {
        /// Color of the mana.
        color: ManaColor,
    },
    /// "Change the target of target spell" (CR 115.7a): each target of the
    /// first target (a spell or ability on the stack) moves to **another**
    /// target that is legal for that spell and matches `to`. It is all of them
    /// or none, and with no such target the original stays, even if it is
    /// illegal by then. Misdirection passes `Filter::Any`; Hydroelectric
    /// Specimen passes `Filter::This` ("to this creature"). A player passes
    /// only `Filter::Any`, since a filter reads objects. Both cards print
    /// "with a single target", which no filter reads yet (#249), so a spell
    /// with more than one target is left alone.
    ChangeTarget {
        /// What the new target must match, beside being legal for the spell.
        to: &'static Filter,
    },
    /// "You may choose new targets for target spell or ability" (CR 115.7d):
    /// any number of the first target's targets may stay as they are, and a
    /// new one must be legal for that spell (Deflecting Swat).
    ChooseNewTargets,
    /// Exchange control of the source and the first target (Gilded
    /// Drake); if no exchange happens (no/illegal target), sacrifice the
    /// source.
    ExchangeControlOrSacrifice,
    /// Exchange control of the first target and the second (Oko, Thief of
    /// Crowns' −5). CR 701.12a–b: if either is gone or both have one
    /// controller, nothing changes hands.
    ExchangeControl,
    /// For each player in `who`, that player chooses up to one matching
    /// permanent they control and it is destroyed (The True
    /// Scriptures I).
    DestroyChosenForPlayers {
        /// Who chooses.
        who: PlayerRel,
        /// What may be destroyed.
        filter: &'static Filter,
    },
    /// Each player in `who` discards `count` cards (their choice).
    DiscardForPlayers {
        /// Who discards.
        who: PlayerRel,
        /// How many cards.
        count: u8,
    },
    /// The chosen player reveals their entire hand; this spell's controller
    /// chooses one matching card for that player to discard (Thoughtseize).
    /// If none match, reveal the hand without asking an impossible choice.
    RevealHandDiscard {
        /// Which cards the controller may choose from the revealed hand.
        filter: &'static Filter,
    },
    /// Discard random cards, using the game's seeded RNG (Mind Twist).
    DiscardRandom {
        /// Players whose hands lose cards.
        who: PlayerRel,
        /// Number of cards, capped by each hand's size.
        count: Amount,
    },
    /// Put all creature cards from all graveyards onto the battlefield
    /// under your control (The True Scriptures III).
    AllGraveyardCreaturesToBattlefield,
    /// Put every card matching `filter` in your graveyard onto the
    /// battlefield, tapped when `tapped`: World Shaper's "return all land
    /// cards from your graveyard to the battlefield tapped", which Lumra,
    /// Bellow of the Woods prints too. Nothing is targeted or chosen; the
    /// cards are the ones there as the effect resolves.
    YourGraveyardToBattlefield {
        /// Which cards.
        filter: &'static Filter,
        /// Whether they enter tapped.
        tapped: bool,
    },
    /// "Earthbend N" (CR 701.66a): "Target land you control becomes a 0/0
    /// land creature with haste in addition to its other types. Put N +1/+1
    /// counters on it. When that land dies or is put into exile, return it
    /// to the battlefield tapped under your control."
    ///
    /// The land is the ability's first target, which the card states as
    /// `TargetReq::one(TargetSpec::Object(&Filter::YOUR_LAND))` beside it.
    /// The animation lasts indefinitely and binds that object, so the land
    /// that comes back is a new object and a plain land again (CR 400.7).
    /// The last sentence is a delayed triggered ability (CR 603.7) whose
    /// controller and source are this ability's (CR 603.7d, 603.7e); it
    /// triggers once (CR 603.7b) and uses the stack.
    Earthbend(u16),
    /// "Return it to the battlefield tapped under your control", where "it"
    /// is a card that has just gone to a graveyard or into exile: the
    /// delayed trigger [`Effect::Earthbend`] leaves behind. `target` is
    /// [`TargetSpec::EventObject`]; nothing is targeted. A card that is no
    /// longer in a graveyard or in exile stays where it is (CR 603.7c).
    ReturnToBattlefieldTapped {
        /// The card (`EventObject`).
        target: TargetSpec,
    },
    /// "Transform this creature" (CR 701.27a): the source turns over to its
    /// other face where it stands. Only a permanent represented by a
    /// transforming double-faced card does (CR 701.27c) — a token copy or a
    /// clone of one turns over nothing.
    TransformSource,
    /// "Transform [this] at the beginning of the next upkeep" (Archangel
    /// Avacyn): a delayed trigger that fires in the next upkeep whoever's
    /// turn it is, and does nothing if the permanent has left or has already
    /// transformed since it was created (CR 701.27f).
    TransformSourceAtNextUpkeep,
    /// Exile the source, then return it to the battlefield under its
    /// owner's control as the given face (transform; Sheoldred's flip,
    /// saga final chapters).
    ExileSelfReturnAsFace {
        /// The face to return as (0 = front).
        face: u8,
    },
    /// Each player in `who` sacrifices a permanent they control matching
    /// the filter (their choice; Sheoldred's Edict).
    SacrificeFilter {
        /// Who sacrifices.
        who: PlayerRel,
        /// What may be sacrificed.
        filter: &'static Filter,
    },
    /// Each player in `who` returns a permanent they control matching the
    /// filter to its owner's hand (their choice; the Ravnica bounce lands —
    /// "when this land enters, return a land you control to its owner's
    /// hand").
    ///
    /// **Nothing here targets.** The card does not print the word, so
    /// CR 115.1 does not apply: the permanent is picked while the ability
    /// resolves (CR 608.2d), hexproof and ward never answer, and nothing
    /// triggers on becoming a target. That is not a detail of the wording —
    /// a bounce land can return a hexproof creature's land, and a targeted
    /// spelling would let a single protected permanent make the whole
    /// ability do nothing.
    ///
    /// **Its owner's hand, never the chooser's**, which CR 400.3 supplies
    /// whatever the card says: a Forest borrowed off another battlefield
    /// goes home rather than joining the borrower's hand.
    ///
    /// Not [`CostPart::ReturnToHand`], which is the same movement bought at
    /// a different moment: a cost is paid while an ability is activated and
    /// an unpayable one means the ability is never announced, while this
    /// happens on resolution and a player with nothing to return simply
    /// does nothing (CR 608.2d "as much as possible"). Quirion Ranger
    /// prints the first sentence and Azorius Chancery the second.
    ///
    /// Mandatory, because the printed sentence is. "You may return …" is
    /// this effect inside a [`Effect::MayDo`], which is where the word
    /// belongs — a flag here would put the decision on the rule instead of
    /// on the card.
    ///
    /// [`CostPart::ReturnToHand`]: crate::CostPart::ReturnToHand
    ReturnChosenToHand {
        /// Who chooses and returns.
        who: PlayerRel,
        /// What may be returned.
        filter: &'static Filter,
    },
    /// "Untap up to N [permanents]" with no "target" (Treachery, Frantic
    /// Search): the controller picks them as the effect resolves, any
    /// controller's, so a permanent with shroud is as good as any.
    UntapChosen {
        /// What may be untapped.
        filter: &'static Filter,
        /// The most that may be.
        count: u8,
    },
    /// Remove all counters from all permanents; the source enters with
    /// that many +1/+1 counters (Thief of Blood).
    DrainAllCountersIntoSelf,
    /// Shuffle your graveyard into your library (Spirit Water Revival's
    /// waterbend outcome).
    ShuffleGraveyardIntoLibrary,
    /// "You may …": the controller is asked, and `effects` run only on a
    /// yes — a choice an effect offers, announced while the effect is
    /// applied (CR 608.2d).
    ///
    /// A block rather than a flag on each effect, because the printed word
    /// covers a whole clause — Ondu Cleric's "you may gain life equal to
    /// the number of Allies you control" is one decision, not one per
    /// operation the clause expands into.
    ///
    /// It is not decoration. Every card here that printed it was written as
    /// if the ability were mandatory, and each has a board where the
    /// automatic answer is the wrong one: a +1/+1 counter on a creature
    /// about to be sacrificed for having the greatest power, life gained
    /// while a "whenever you gain life" trigger is pointed the other way.
    /// The whole point of a "may" is that the player is allowed to decline,
    /// so the engine has to ask.
    MayDo {
        /// What happens on a yes.
        effects: &'static [Effect],
    },
    /// "You may …. Do this only once each turn." (The Reaper, King No More.)
    ///
    /// The ability still triggers every time; what is limited is the
    /// optional action. A yes uses the turn's one go and a no does not, so
    /// the question is asked again the next time the ability resolves. A
    /// source that has used it this turn is not asked at all, and neither is
    /// one whose action has become impossible (CR 608.2d): the card to put
    /// onto the battlefield has left the graveyard. The limit is kept per
    /// object, so a source that left the battlefield and came back is a new
    /// object with a fresh turn (CR 400.7).
    MayDoOnceEachTurn {
        /// What happens on a yes.
        effects: &'static [Effect],
    },
    /// "When you do, …": creates a reflexive triggered ability
    /// (CR 603.12). It is written as the **last** op of a list that
    /// resolves off the stack, directly after the action it waits for. It
    /// checks that action against what this resolution has already done,
    /// and creates nothing when the action did not happen.
    ///
    /// Two nearby shapes looked like this one and are not.
    /// - It is not a [`crate::ability::Trigger`]. CR 603.7a says the
    ///   ability "won't trigger until it has actually been created". Brokers
    ///   Hideout's half, written as a leaves-the-battlefield trigger, fired
    ///   on a bounce in response. CR 603.12's Manticore example rules that
    ///   reading out.
    /// - It is not the rest of the action's own list, or an `If…` beside
    ///   it. Those run inside the same resolution. They put nothing on the
    ///   stack, give nobody priority, and cannot choose a target after the
    ///   action. Eden's "another target permanent card" may take a card its
    ///   own mill has just put into the graveyard.
    ///
    /// `lints::every_reflexive_sits_where_it_can_trigger` holds the
    /// placement. The engine counts any departure of the source by effect
    /// as the sacrifice, and that is exact only in that shape.
    Reflexive {
        /// The event, checked against this resolution's earlier events.
        when: ReflexiveEvent,
        /// What the triggered ability does when it resolves.
        effects: &'static [Effect],
        /// Its target. It is chosen as the ability is put on the stack
        /// (CR 603.3d, which applies CR 601.2c), not while the resolution
        /// that created it runs. It is `Option<TargetSpec>` and not a
        /// [`TargetReq`] because that is what a synthetic trigger carries.
        /// The spec says how many: exactly one object, or
        /// [`TargetSpec::ObjectOfEachOpponent`]'s up to one per opponent
        /// (The Balrog of Moria), asked opponent by opponent as the printed
        /// path asks it.
        target: Option<TargetSpec>,
    },
    /// Branch on whether the spell was kicked (paid its additional cost).
    IfKicked {
        /// Effects when kicked.
        then: &'static [Effect],
        /// Effects otherwise.
        otherwise: &'static [Effect],
    },
    /// Branch on any [`crate::Condition`] — "draw a card **if you control
    /// an artifact**", "add {B}{B}{B}{B}{B} **instead if** there are seven
    /// or more cards in your graveyard".
    ///
    /// The general form of the six neighbours around it, and the reason it
    /// is worth having beside them: a `Condition` is already the vocabulary
    /// an activation gate and an intervening `if` are written in, and every
    /// sentence added there was readable in those two positions and
    /// unsayable in an effect list. Threshold gated Barbarian Ring's
    /// activation and could not make Cabal Ritual's mana bigger; the same
    /// count, the same seat, two different answers.
    ///
    /// The condition is read **as this effect runs**, not when the ability
    /// was put on the stack: CR 608.2 resolves an instruction in the order
    /// it is written, so a card whose earlier half filled the graveyard
    /// asks the later half about the graveyard it just filled.
    ///
    /// `otherwise` is a list and not an `Option`, because "add {U}, or {B}
    /// instead" and "draw a card if you control an artifact" are the same
    /// shape with an empty else — and an `Option` would put the difference
    /// in the type rather than in the card.
    IfCondition {
        /// The sentence asked.
        condition: crate::Condition,
        /// Effects when it holds.
        then: &'static [Effect],
        /// Effects when it does not.
        otherwise: &'static [Effect],
    },
    /// "…if this is the first time this ability has resolved this turn. If
    /// it's the second time, …. If it's the third time, …." (Omnath, Locus
    /// of Creation.) The nth effect runs as this ability resolves for the
    /// nth time this turn, and a resolution past the end of the list does
    /// nothing.
    ///
    /// The count is the ability's own, kept per object and ability index in
    /// the turn's tally (`GameState::ability_fires` in the engine), so a
    /// source that left the battlefield and came back starts over
    /// (CR 400.7). It is counted as this effect runs, which is the ability
    /// resolving when the effect is the whole of it, as Omnath's is.
    NthResolutionThisTurn {
        /// One effect per resolution, the first time first.
        effects: &'static [Effect],
    },
    /// The source gains the prepared marker (Emeritus of Woe's
    /// re-prepare trigger).
    BecomePrepared,
    /// "… target … if it's [filter]": the effects run only when the first
    /// target, as it is when this runs, matches `filter` (Prismatic Ending:
    /// "Exile target nonland permanent if its mana value is less than or
    /// equal to the number of colors of mana spent to cast this spell").
    ///
    /// The condition is not a targeting restriction: the target is chosen
    /// by the requirement alone (CR 601.2c), before the costs are paid
    /// (CR 601.2h) that converge counts, and a target that fails `filter`
    /// is still a legal one — the spell resolves and does nothing to it.
    IfTargetMatches {
        /// What the target has to be.
        filter: &'static crate::Filter,
        /// Effects when it is.
        then: &'static [Effect],
    },
    /// Branch when at least N creatures died this turn (Emeritus of
    /// Woe's re-prepare condition).
    IfCreaturesDiedAtLeast {
        /// Threshold.
        n: u32,
        /// Effects when the condition holds.
        then: &'static [Effect],
    },
    /// Branch: "if this is the `times`th time this ability has resolved this
    /// turn" (Nissa, Resurgent Animist: "Then if this is the second time
    /// this ability has resolved this turn, …"). The engine counts every
    /// resolution of an ability of one object in its per-turn record, this
    /// one included, and the branch runs when the count is exactly `times`:
    /// a third resolution is not the second. The object is the source as it
    /// is now (CR 400.7), so a Nissa that left and came back is a new object
    /// whose abilities start counting again.
    IfResolvedTimesThisTurn {
        /// The count at which the branch runs.
        times: u32,
        /// Effects when it does.
        then: &'static [Effect],
    },
    /// Branch: you didn't lose life this turn (Luminarch Ascension).
    IfNotLostLifeThisTurn {
        /// Effects when the condition holds.
        then: &'static [Effect],
    },
    /// Branch: you control a `filter`-matching permanent with the
    /// greatest cmc among `filter`-matching permanents (or tied; Padeem).
    IfControlGreatestCmc {
        /// The comparison class.
        filter: &'static Filter,
        /// Effects when the condition holds.
        then: &'static [Effect],
    },
    /// Branch: the source has no counters of this kind left.
    ///
    /// "**If** there are no depletion counters on this land, sacrifice it"
    /// — the second half of the sentence whose first half is the mana, and
    /// an ordinary effect rather than a trigger or a state-based action,
    /// because that is how the card prints it. The ability removes a
    /// counter as a cost ([`crate::CostPart::RemoveCounterSelf`]), makes its
    /// mana, and then asks this; a land activated for the last time is
    /// therefore in the graveyard before anybody gets priority back
    /// (CR 605.3b).
    ///
    /// Counted over every `//! Oracle:` header on 2026-09-16, the pool
    /// prints the sentence on exactly **six** cards: the five Mercadian
    /// Masques depletion lands and Gemstone Mine, which says "mining"
    /// where they say "depletion" and is otherwise the same card. That is
    /// why the kind is a parameter and the "no" is not: Magic prints the
    /// comparison against zero and no other, so a threshold field would be
    /// a number no card has ever written.
    ///
    /// The source must still be there for the condition to hold. "There
    /// are no counters on this land" presupposes the land, and a permanent
    /// that has already left has no counters in exactly the way an empty
    /// set does not — reading the absent object as zero would sacrifice
    /// something twice.
    IfNoCountersOnSelf {
        /// Which counter has to have run out.
        kind: CounterKind,
        /// Effects when it has (`&[Effect::SacrificeSelf]`, on all six).
        then: &'static [Effect],
    },
    /// Branch on the event object's power (Tribute to the World Tree):
    /// `then` when power >= `n`, else `otherwise`.
    IfEventPowerAtLeast {
        /// Threshold.
        n: i16,
        /// Effects when power >= n.
        then: &'static [Effect],
        /// Effects otherwise.
        otherwise: &'static [Effect],
    },
    /// Twice X (Heliod's Intervention lifegain mode) — helper amount.
    GainLifeDoubleX,
    /// Search your library for matching cards (server-side filtered).
    ///
    /// One [`Find`] per card the search may produce, in the order the card
    /// text names them: Cultivate's "put one onto the battlefield tapped and
    /// the other into your hand" is two finds with different destinations,
    /// and Rampant Growth is one. A single `dest`/`tapped` pair used to be
    /// the whole vocabulary here, which is why every card that fetches two
    /// lands was inexpressible.
    SearchLibrary {
        /// What to find.
        filter: &'static Filter,
        /// Where each found card goes, positionally. `finds.len()` is how
        /// many cards may be found.
        finds: &'static [Find],
        /// Whether you may find fewer than `finds.len()` ("up to", "you may").
        optional: bool,
    },
    /// Scry N.
    Scry {
        /// How many.
        amount: Amount,
    },
    /// Surveil N (CR 701.25a).
    ///
    /// Its own variant rather than a flag on [`Self::Scry`], because the two
    /// differ in where the cards a player does *not* keep end up — and a
    /// graveyard is a public zone somebody else's card reads. Delirium,
    /// threshold, escape and every "whenever a creature card is put into a
    /// graveyard" trigger can see a surveil and can never see a scry.
    ///
    /// No `SurveilFor`: every one of the reference corpus's 228 surveil
    /// lines is the controller's own library, and a card that made somebody
    /// else surveil would be putting cards into *their* graveyard, which is
    /// a different sentence rather than a parameter.
    Surveil {
        /// How many.
        amount: Amount,
    },
    /// A relative player scries N (Jace's +2).
    ScryFor {
        /// Who.
        player: PlayerRel,
        /// How many.
        amount: Amount,
    },
    /// Exile all cards from a player's library, then they shuffle their
    /// hand into their library (Jace's ultimate).
    ExileLibraryAndShuffleHand {
        /// Who.
        player: PlayerRel,
    },
    /// All objects matching a filter get P/T set to computed values until
    /// a duration ends (Karn's animation).
    SetPTFilter {
        /// Which objects.
        filter: &'static Filter,
        /// New power (may be computed, e.g. `TargetCmc`).
        power: Amount,
        /// New toughness.
        toughness: Amount,
        /// How long.
        duration: crate::static_ability::Duration,
    },
    /// Mill cards.
    Mill {
        /// How many.
        amount: Amount,
        /// Who.
        target: PlayerRel,
    },
    /// Add mana to your pool.
    ///
    /// Prefer the constructors — [`Effect::mana`], [`Effect::mana_choice`],
    /// [`Effect::mana_of_any_color`] — which read like the printed line.
    AddMana {
        /// Which colors are available.
        source: ManaSource,
        /// How much (dynamic amounts evaluate on resolution: Harabaz Druid
        /// produces one per Ally).
        amount: Amount,
        /// Whether each mana may be a different color (filter lands).
        combination: bool,
        /// What the mana may be spent on, if restricted.
        restriction: Option<ManaRestriction>,
    },
    /// Add a subtype-granting note — placeholder for M2 (changeling etc.).
    GrantSubtype {
        /// Subtype.
        subtype: SubtypeId,
    },
    /// Put counters on the first target (or the source when no target).
    AddCounter {
        /// Counter kind.
        kind: CounterKind,
        /// How many.
        amount: Amount,
    },
    /// Put counters on every object matching a filter (Kazandu
    /// Blademaster's rally).
    AddCounterFilter {
        /// Which objects.
        filter: &'static Filter,
        /// Counter kind.
        kind: CounterKind,
        /// How many per object.
        amount: Amount,
    },
    /// Double the number of counters of a kind on every permanent a filter
    /// matches (Bristly Bill's "double the number of +1/+1 counters on each
    /// creature you control").
    ///
    /// CR 701.10e: each gets as many of those counters as it already has,
    /// and that is *putting* counters, so a Doubling Season has its say
    /// (Bristly Bill's ruling) — the counters go through the same door as
    /// [`Self::AddCounterFilter`]'s.
    DoubleCountersFilter {
        /// Which permanents.
        filter: &'static Filter,
        /// Which counters.
        kind: CounterKind,
    },
    /// Return a target object (battlefield or stack) to its owner's hand.
    ReturnToHand {
        /// What.
        target: TargetSpec,
    },
    /// Return all objects matching a filter to their owners' hands.
    ReturnAllToHand {
        /// What.
        filter: &'static Filter,
        /// Only objects controlled by opponents (Cyclonic Rift style).
        opponents_only: bool,
    },
    /// "Exile all [permanents]" (Farewell): every permanent `filter`
    /// matches as this resolves. Nothing is targeted (CR 115.1a names a
    /// target by the word), so hexproof and protection do not stop it.
    ExileAll {
        /// What.
        filter: &'static Filter,
    },
    /// "Choose a creature you control. It gains indestructible until end of
    /// turn." (Final Showdown): as this resolves its controller chooses one
    /// permanent they control that `filter` matches (CR 608.2d), and `then`
    /// happens to it — the chosen permanent is what `Filter::This` names
    /// inside `then`. A choice and not a target (CR 115.1a), so hexproof
    /// does not stop it; with nothing to choose, `then` does nothing
    /// (CR 609.3).
    ///
    /// `then` is run as a nested list with the choice as its object, and a
    /// nested list that stops for a question hands the rest of itself back
    /// to the outer one, which does not know the choice — so `then` holds
    /// only effects that ask nothing (`lints::chosen_then_fault`).
    ChooseYoursThen {
        /// What may be chosen, among the permanents its controller controls.
        filter: &'static Filter,
        /// What happens to the chosen one.
        then: &'static [Effect],
    },
    /// Destroy all objects matching a filter (wraths).
    DestroyAll {
        /// What.
        filter: &'static Filter,
        /// "They can't be regenerated" (CR 701.19c) — see
        /// [`Effect::Destroy::no_regen`].
        no_regen: bool,
    },
    /// "…and all other permanents with the same name as that permanent"
    /// (Maelstrom Pulse): destroy every permanent other than the object
    /// `target` names that shares its name.
    ///
    /// The name is the target's **current** one, read as this resolves, so
    /// it goes before the effect that destroys the target: a Clone copying
    /// a Llanowar Elves is named Llanowar Elves only while it is on the
    /// battlefield. A nameless permanent (a face-down one, CR 708.2a) shares
    /// a name with nothing (CR 201.2a), so it sweeps nothing. The sweep
    /// targets nothing but the one permanent, so hexproof or protection on
    /// the others does not stop it.
    DestroyOthersNamedLike {
        /// The permanent whose name is swept.
        target: TargetSpec,
    },
    /// Regenerate a permanent (CR 701.19a): the next time it would be
    /// destroyed this turn, instead remove all damage marked on it, its
    /// controller taps it, and if it is attacking or blocking it is
    /// removed from combat.
    ///
    /// A **count** on the object rather than a flag, because each
    /// resolution creates its own shield and a creature regenerated twice
    /// survives being destroyed twice.
    ///
    /// [`TargetSpec::ThisObject`] is the self case — "Regenerate this
    /// creature" names no target (CR 115.1), so Thrun and Lotleth Troll do
    /// not go through the targeting machinery while Yavimaya Hollow's
    /// "regenerate target creature" does. One variant rather than a
    /// `RegenerateSelf` beside it, because `spec_object` already answers
    /// both from the same field.
    Regenerate {
        /// Which permanent gets the shield.
        target: TargetSpec,
    },
    /// Exile all cards from a player's graveyard (Bojuka Bog).
    ExileGraveyard {
        /// Whose graveyard.
        player: PlayerRel,
    },
    /// Put a graveyard card on top of its owner's library (Volrath's).
    GraveyardToTop {
        /// What (`CardInGraveyard`).
        target: TargetSpec,
    },
    /// Return a graveyard card to its owner's hand (Archaeomancer).
    GraveyardToHand {
        /// What (`CardInGraveyard`).
        target: TargetSpec,
    },
    /// "Return to your hand all [filter] cards in your graveyard" (Garna,
    /// the Bloodflame). No target: every matching card in the controller's
    /// graveyard as the effect resolves.
    GraveyardAllToHand {
        /// Which cards.
        filter: &'static Filter,
    },
    /// Put a graveyard card onto the battlefield (reanimation).
    GraveyardToBattlefield {
        /// What: a `CardInGraveyard` the spell or ability targeted, or an
        /// `EventObject` for the card that just died (Journey to Eternity's
        /// "return it to the battlefield", which names no target).
        target: TargetSpec,
        /// Under whose control it arrives.
        ///
        /// Every reanimation spell in this pool prints "under your control"
        /// and takes the `false`; undying (CR 702.93a), persist
        /// (CR 702.79a) and Luminous Broodmoth print "under its **owner's**
        /// control". The two differ only while somebody else is controlling
        /// the creature that dies — which is exactly the moment a player
        /// would notice, because a stolen creature with undying comes home.
        owner_control: bool,
        /// "…with a +1/+1 counter on it" (undying) or "-1/-1" (persist).
        ///
        /// A rider on one sentence rather than a second effect, for the
        /// reason [`Effect::Destroy::no_regen`] is one: the card prints a
        /// single clause and splitting it would make two doors where the
        /// rules have one. It goes through `replacement::put_counters`, so
        /// a counter doubler has its say (CR 614.16).
        counters: Option<(CounterKind, u16)>,
    },
    /// Create a token that gets +P/+T for each filter-matching permanent
    /// you control (Urza's Saga's Construct; registered as its own
    /// continuous effect).
    CreateTokenPtPerCount {
        /// The token.
        token: &'static TokenDef,
        /// What to count.
        filter: &'static Filter,
        /// Power per match.
        p: i16,
        /// Toughness per match.
        t: i16,
    },
    /// Create a token under your control.
    CreateToken {
        /// What.
        token: &'static TokenDef,
    },
    /// Create N tokens under your control (Aang and Katara).
    CreateTokenN {
        /// What.
        token: &'static TokenDef,
        /// How many.
        amount: Amount,
    },
    /// Create a token under the first target's controller (Crib Swap).
    CreateTokenForTargetController {
        /// What.
        token: &'static TokenDef,
    },
    /// Amass N (CR 701.47): put N +1/+1 counters on an Army you control, or
    /// create `token` first if you control none.
    ///
    /// `subtype` is the type the mechanic names — "amass Orcs 1" makes the
    /// Army an Orc Army in addition to its other types (CR 701.47a), whether
    /// it was just created or was already on the battlefield. The token comes
    /// from the card rather than the engine because the rules kernel does not
    /// know the token registry, and because a token without a registry entry
    /// has no art key.
    Amass {
        /// The Army token to create when you control no Army.
        token: &'static TokenDef,
        /// The creature type the Army also becomes.
        subtype: SubtypeId,
        /// How many counters.
        amount: u16,
    },
    /// Put the source on top of its owner's library (Sensei's Divining Top).
    PutSourceOnTopOfLibrary,
    /// Create a token that's a copy of a target permanent (Rite of
    /// Replication, Progenitor Mimic).
    CreateTokenCopyOf {
        /// What to copy (first target when set, else the source).
        target: Option<TargetSpec>,
        /// Extra copies when the spell was kicked (Rite of Replication: 4
        /// bonus tokens for a total of 5).
        kicked_bonus: u8,
    },
    /// Populate (CR 701.36): choose a creature token you control as this
    /// resolves and create a token that's a copy of it; none, no token
    /// (701.36b). A choice and not a target, so shroud does not stop it
    /// (Nesting Dovehawk).
    Populate,
    /// "Create a token that's a copy of target …, except …" with the
    /// exceptions as copy modifications (CR 707.9), and, when
    /// `sacrifice_at_next_end_step`, "Sacrifice it at the beginning of the
    /// next end step" as a delayed trigger that follows the token and no
    /// other object (Kiki-Jiki, Mirror Breaker; Reflection of Kiki-Jiki).
    /// Reads the first target.
    CreateTokenCopyOfTarget {
        /// The "except" clauses, applied to the copiable values before the
        /// token is made.
        mods: &'static [crate::ability::CopyMod],
        /// "Sacrifice it at the beginning of the next end step."
        sacrifice_at_next_end_step: bool,
    },
    /// "Create a token that's a copy of it, except …" where "it" is the
    /// source card itself (eternalize and embalm, CR 702.129a, 702.128a):
    /// the card was exiled to pay the cost, and the copy takes its copiable
    /// values there (CR 707.2) with `mods` applied (CR 707.9).
    CreateTokenCopyOfSource {
        /// The "except" clauses.
        mods: &'static [crate::ability::CopyMod],
    },
    /// Create a token that's a copy of the creature the source is attached
    /// to (Helm of the Host).
    CreateTokenCopyOfEquipped {
        /// Extra copies when the spell was kicked.
        kicked_bonus: u8,
        /// Copy modifications ("isn't legendary", "gains haste").
        mods: &'static [crate::ability::CopyMod],
    },
    /// Create a token that's a copy of the first creature token you
    /// control (populate; no-op if none).
    CreateTokenCopyOfFirstToken,
    /// A relative player puts a filtered card from their hand on the
    /// bottom of their library (Vendilion Clique).
    BottomCardFromHand {
        /// Whose hand.
        player: PlayerRel,
        /// Which cards may be chosen.
        filter: &'static Filter,
    },
    /// Copy a spell on the stack (Double Major, Jin-Gitaxias). The copy
    /// goes on the stack under your control; you may choose new targets
    /// (M3 protocol choice; currently same targets).
    /// Copy the first target (a spell on the stack) with modifications
    /// ("except it isn't legendary").
    CopyTargetSpell {
        /// Copy modifications.
        mods: &'static [crate::ability::CopyMod],
    },
    /// "Copy target activated or triggered ability you control. You may
    /// choose new targets for the copy." (Vantress Visions.) The first
    /// target is an ability on the stack (`TargetSpec::AbilityOnStack`).
    ///
    /// The copy is put on the stack under your control with every decision
    /// made for the original: its mode, targets, X and what paid its costs
    /// (CR 707.10), and the same source (CR 707.10b). It is neither
    /// activated nor triggered (CR 707.10), so nothing that watches for
    /// either sees it. Its controller may then leave any number of its
    /// targets unchanged and change the rest to legal ones (CR 707.10c),
    /// one target at a time as `ChooseNewTargets` asks.
    CopyTargetAbility,
    /// One copy of the spell a keyword's cast trigger is about: "copy it",
    /// for replicate's "copy it for each time its replicate cost was paid.
    /// If the spell has any targets, you may choose new targets for any of
    /// the copies" (CR 702.56a).
    ///
    /// Written by the engine and never by a card: a card prints
    /// `replicate = Some(…)` on its face, and the trigger the engine puts on
    /// the stack for it lists this once for each payment, so the count is
    /// fixed as the spell is cast. The spell is the trigger's first target
    /// (its implicit one), copied as it last existed if it has left the
    /// stack by then (CR 608.2h). The copy is put on the stack with every
    /// decision made for the original (CR 707.10), and its controller may
    /// then keep or change each of its targets, one at a time
    /// (CR 707.10c), as `ChooseNewTargets` asks.
    CopyThisSpell,
    /// Attach the source (equipment/aura) to a target permanent.
    AttachSelf {
        /// To what.
        target: TargetSpec,
    },
    /// Look at the top N cards of your library and put them back in any
    /// order.
    ReorderTopLibrary {
        /// How many.
        count: u8,
    },
    /// Shockland entry: you may pay N life; if you don't, the source
    /// enters tapped (yes/no choice).
    PayLifeOrEnterTapped {
        /// Life to pay.
        amount: u16,
    },
    /// A player may pay generic mana; if they don't, run `effect` (Rhystic
    /// Study, Esper Sentinel, Smothering Tithe, ward).
    ///
    /// The amount is an [`Amount`] rather than a number because one of
    /// those cards does not print one: Esper Sentinel taxes `{X}` where X
    /// is its own power, and writing `1` there made it a `{1}` tax that no
    /// anthem, counter or equipment could move. It read as correct because
    /// a 1/1 with nothing on it does cost `{1}`.
    PlayerMayPayOr {
        /// Who decides.
        player: PlayerRel,
        /// Generic mana to pay, evaluated when the ability resolves.
        mana: Amount,
        /// What happens when they don't pay.
        effect: &'static Effect,
    },
    /// A player may pay life; declining or being unable to pay runs `effect`.
    /// The amount is evaluated on resolution, including a ward source's power.
    PlayerMayPayLifeOr {
        /// Who pays.
        player: PlayerRel,
        /// Life cost, evaluated when resolving.
        life: Amount,
        /// Unpaid consequence.
        effect: &'static Effect,
    },
    /// "… unless you <pay something that is not mana>."
    ///
    /// The sibling of [`Effect::PlayerMayPayOr`] and deliberately not a
    /// field on it: that one charges generic mana in an amount only
    /// resolution knows (Esper Sentinel's tax is its own power), which a
    /// `ManaCost` cannot hold, while this one charges a cost the player pays
    /// by naming an object — a Karoo land's "return an untapped Plains you
    /// control", a Command Bridge's "tap an artifact or land". Folding both
    /// into one variant would mean a price with two halves, and no card in
    /// this pool prints one.
    ///
    /// There is no separate yes-or-no question. The player is asked to name
    /// what pays, and naming nothing is how they decline — so a cost nobody
    /// can pay asks nothing at all, which is what a Karoo entering under a
    /// controller with no untapped Plains should do.
    PlayerMayPayCostOr {
        /// Who decides.
        player: PlayerRel,
        /// What they may pay. One part, named by the object that pays it.
        cost: &'static CostPart,
        /// What happens when they don't pay.
        effect: &'static Effect,
    },
    /// "You may pay {1}. If you do, you gain 1 life." (Crystal Rod and its
    /// four siblings, Soul Net, Mana Vault's upkeep untap.)
    ///
    /// The mirror of [`Effect::PlayerMayPayOr`], and a variant of its own
    /// rather than a flag on it, because the two run their effect on
    /// opposite answers and a flag read the wrong way round is a card that
    /// hands out its reward for nothing. The question and the payment are
    /// the same ones: a yes-or-no put as the ability resolves (CR 608.2d),
    /// paid in mana that the player may make right then (CR 605.3a).
    PlayerMayPayThen {
        /// Who decides and pays.
        player: PlayerRel,
        /// Generic mana to pay, evaluated when the ability resolves.
        mana: Amount,
        /// What happens when they pay.
        effects: &'static [Effect],
    },
    /// Create a continuous effect (Giant Growth style): applies `modifier`
    /// on `layer` to `filter` for `duration`. `filter = This` binds to the
    /// first target.
    CreateContinuousEffect {
        /// The layer it applies in.
        layer: crate::static_ability::Layer,
        /// Which objects are affected (`This` = first target).
        filter: &'static Filter,
        /// What changes.
        modifier: crate::static_ability::Modifier,
        /// How long it lasts.
        duration: crate::static_ability::Duration,
    },
    /// Change who controls a target permanent (Gilded Drake exchange,
    /// Homeward Path restore).
    ChangeController {
        /// Who gains control.
        new_controller: PlayerRel,
    },
    /// Each player gains control of all creatures they own (Homeward
    /// Path).
    AllCreaturesToOwner,
    /// Choose left or right. Each player receives the nonland permanents
    /// of that neighbour, except the source (Aminatou −6).
    ControlRotation,
    /// Phase a target permanent out (Clever Concealment).
    PhaseOut {
        /// What phases out (first target when set, else the source).
        target: Option<TargetSpec>,
    },
    /// Exile a target with a link to the source, so that a later ability of
    /// the source can find it among the cards "exiled with" it (CR 607.2a),
    /// for as long as `until` says.
    ///
    /// `None` is an exile with no end of its own. The card stays until an
    /// effect of the source brings it back (Safe Haven and Endless Sands,
    /// [`Effect::ReturnLinkedToBattlefield`]) or for good (Skyclave
    /// Apparition). `Some` is an "until" sentence (CR 610.3): the return is
    /// the second half of the same effect and not a triggered ability, so
    /// it happens the moment the event does, uses no stack, and puts the
    /// card back under its owner's control (CR 610.3c). If the event has
    /// already happened when the exile would, the card does not move
    /// (CR 610.3a, 610.3b).
    ///
    /// Spelled [`Effect::exile_linked`] and [`Effect::exile_until`].
    ExileLinked {
        /// What.
        target: TargetSpec,
        /// The event that ends the exile, when the sentence names one.
        until: Option<ExileUntil>,
    },
    /// Exile every target, each **exiled with** the source (CR 406.6):
    /// "Exile up to two target cards from a single graveyard" (Unlicensed
    /// Hearse), whose power and toughness count them
    /// (`PtCount::ExiledWithThis`). Nothing brings them back, which is what
    /// keeps it apart from [`Effect::ExileLinked`]'s "until …" exile and its
    /// rider, which a leaving host and a new monarch both read.
    ExileTargetsWithSource,
    /// Return everything exiled with a link to the source to the
    /// battlefield under its owner's control.
    ReturnLinkedToBattlefield,
    /// Create a token under the *owner* of the card exiled with a link to
    /// the source, with power/toughness set to that card's mana value
    /// (Skyclave Apparition's Illusion).
    CreateTokenFromLinked {
        /// The token to create (power/toughness are overridden by the
        /// linked card's mana value).
        token: &'static TokenDef,
    },
    /// Sacrifice the source permanent (evoke).
    SacrificeSelf,
    /// Register a delayed "pay or lose" trigger at your next upkeep
    /// (Pact of Negation).
    PayCostOrLoseLater {
        /// The mana cost to pay at your next upkeep.
        cost: ManaCost,
    },
    /// The controller gets an emblem with the given abilities
    /// (planeswalker ultimates).
    CreateEmblem {
        /// The emblem's abilities.
        abilities: &'static [crate::ability::AbilityDef],
    },
    /// A player becomes the monarch (CR 724): `You` for Palace Jailer, and
    /// `ControllerOfTarget` for the monarch's own "its controller becomes
    /// the monarch", which the engine resolves with the creature that dealt
    /// the damage as its first object.
    BecomeMonarch(PlayerRel),
    /// A relative player may search their library for a basic land onto
    /// the battlefield tapped, then shuffle (Path to Exile).
    OptionalBasicLandSearchFor {
        /// Who may search.
        player: PlayerRel,
    },
    /// A search [`Effect::SearchLibrary`] cannot say: somebody else's
    /// library, somebody else doing the searching, or a mana-value bound
    /// that only the resolution knows.
    ///
    /// A sibling rather than three more fields on `SearchLibrary`, which
    /// thirty-odd card files and a reader write as a literal. Every search
    /// that fits `SearchLibrary` keeps writing it.
    ///
    /// - Boseiju, Who Endures and Assassin's Trophy: "**that player** may
    ///   search **their** library for …, put it onto the battlefield" —
    ///   `library: ControllerOfTarget`, `owner_searches: true`, `optional`.
    /// - Bribery: "search **target opponent's** library for a creature card
    ///   and put that card onto the battlefield **under your control**. Then
    ///   that player shuffles." — `library: Chosen`, `owner_searches: false`.
    /// - Eldritch Evolution, Neoform, Birthing Pod: "a creature card with
    ///   mana value X or less, where X is 2 plus the sacrificed creature's
    ///   mana value" — `mana_value`, read as the search begins.
    ///
    /// Whoever searches, the library searched is the one shuffled
    /// afterwards, and a find that goes to a hand goes to the searcher's.
    SearchLibraryOf {
        /// Whose library is searched.
        library: PlayerRel,
        /// Whether that library's owner searches it and gets what they find
        /// (true), or you search it and what you find is yours to control
        /// (false). With `library: You` the two are the same.
        owner_searches: bool,
        /// What to find.
        filter: &'static Filter,
        /// A mana-value bound read at resolution, on top of `filter`.
        mana_value: Option<ManaValueBound>,
        /// Where each found card goes, positionally, as for `SearchLibrary`.
        finds: &'static [Find],
        /// Whether fewer than `finds.len()` may be found ("up to", "may").
        optional: bool,
    },
    /// "Search your library for up to `count` `filter` cards", where the
    /// count is a number only the resolution knows (Nylea's Intervention:
    /// "up to X land cards, reveal them, put them into your hand, then
    /// shuffle"). Every card found goes where `find` says.
    ///
    /// A sibling for the reason [`Self::SearchLibraryOf`] is one:
    /// `SearchLibrary`'s `finds` is a count written into the card, one
    /// [`Find`] per card, and no slice can be X long. Always "up to": a
    /// search for a number of cards the searcher did not choose is not a
    /// sentence any printing uses. A reveal follows the rule every search
    /// follows (narrower than "a card", ending in a hidden zone).
    SearchLibraryUpTo {
        /// What to find.
        filter: &'static Filter,
        /// How many at most, read as the search begins.
        count: Amount,
        /// Where each card found goes (a reference, so the engine can hand
        /// it to the search as the one-element list every search reads).
        find: &'static Find,
    },
    /// "Search your library for up to `up_to` `filter` cards with different
    /// names and reveal them. An opponent chooses `chosen` of those cards.
    /// Put the chosen cards into your graveyard and the rest into your hand.
    /// Then shuffle." (Realms Uncharted: land cards, up to four, two chosen.)
    ///
    /// Different names are a property of the answer, and the search offers
    /// one card per name so that every answer has it: two copies of a card
    /// in a library are the same card to every rule this sentence reads.
    /// With several opponents the controller names the one who chooses, as
    /// CR 700.2e has them do for a mode another player chooses. Finding
    /// `chosen` or fewer leaves nothing to choose: every card found is
    /// chosen.
    SearchOpponentSplits {
        /// What to find.
        filter: &'static Filter,
        /// How many at most.
        up_to: u8,
        /// How many of those the opponent sends to the graveyard.
        chosen: u8,
    },
    /// All objects matching a filter get computed P/T modifiers, and
    /// optionally keywords, until a duration ends (Toxic Deluge: `-X/-X`
    /// on all creatures; Overrun: `+3/+3` and trample on your team).
    ///
    /// `Filter::This` here means the *source*, as it does in every other
    /// filter position — see [`Effect::PumpTarget`] for the targeted case.
    PumpFilter {
        /// Which objects are pumped.
        filter: &'static Filter,
        /// Whose permanents, when the printed sentence names a player
        /// rather than the board — "creatures **target player** controls
        /// get -2/-2".
        ///
        /// `None` is every object the filter matches, which is what "all
        /// creatures" means. It is a field here rather than a [`Filter`]
        /// variant because a filter is evaluated against an object and the
        /// two things it is told, `you` and `this`, are the ability's
        /// controller and its source: the seat a spell *chose* is neither,
        /// and is not a characteristic of anything. The same argument
        /// [`TargetSpec::AnyTarget`] already makes about players.
        controlled_by: Option<PlayerRel>,
        /// Power modifier (may be negative/X-driven).
        power: Amount,
        /// Toughness modifier (may be negative/X-driven).
        toughness: Amount,
        /// Keywords granted for the same duration ([`KeywordSet::EMPTY`]
        /// for a plain pump).
        keywords: KeywordSet,
        /// How long.
        duration: crate::static_ability::Duration,
    },
    /// The spell's or ability's targets get P/T modifiers, and optionally
    /// keywords, until a duration ends (Giant Growth, Titanic Growth,
    /// Brute Force; with keywords, Might of Old Krosa or Rush of Blood).
    ///
    /// Distinct from [`Effect::PumpFilter`] because "the target" is not a
    /// characteristic and no `Filter` can name it. It applies to *every*
    /// target, so a spell with `TargetReq` for two creatures pumps both,
    /// and it takes [`Amount`]s, so `+X/+X` is expressible where
    /// [`Effect::CreateContinuousEffect`]'s fixed [`crate::Modifier`] is
    /// not.
    PumpTarget {
        /// Power modifier (may be negative/X-driven).
        power: Amount,
        /// Toughness modifier (may be negative/X-driven).
        toughness: Amount,
        /// Keywords granted for the same duration ([`KeywordSet::EMPTY`]
        /// for a plain pump).
        keywords: KeywordSet,
        /// How long.
        duration: crate::static_ability::Duration,
    },
    /// "target creature gains protection from the color of your choice
    /// until end of turn" (Sejiri Steppe). The color is chosen as the
    /// effect resolves (CR 608.2d asks it then, not on activation), by its
    /// controller, among the five; the protection is a layer-6 grant
    /// (CR 613.1f) on the first target for `duration`.
    ProtectionFromChosenColor {
        /// How long.
        duration: crate::static_ability::Duration,
    },
    /// "Tap all creatures your opponents control" (Cryptic Command): every
    /// permanent `filter` matches as this resolves becomes tapped (CR
    /// 701.26a). Nothing is targeted (CR 115.1a names a target by the
    /// word), so hexproof and protection do not stop it, and a permanent
    /// already tapped stays as it is.
    TapAll {
        /// What.
        filter: &'static Filter,
    },
    /// "Exile the top card of that player's library. Until end of turn, you
    /// may cast that card." (Ragavan, Nimble Pilferer): the top card of each
    /// library `who` names goes to its owner's exile face up, and the
    /// controller may cast it this turn, paying its costs (a
    /// `PlayPermission` in the engine, cast only: a land exiled this way is
    /// not played). Nothing is targeted, and an empty library exiles
    /// nothing.
    ExileTopMayCast {
        /// Whose library: the owner's relation to you.
        who: PlayerRel,
    },
}

impl Effect {
    /// "Draw a card." / "Draw three cards."
    ///
    /// # The verbs, and where they stop
    ///
    /// This and the seven below are the printed sentence as one call. The
    /// precedent is [`Effect::mana`] directly underneath: it has 219 uses in
    /// the pool against **zero** raw `AddMana` literals, so a verb that
    /// reads like the card is adopted without anybody being told to.
    ///
    /// Two rules keep that from becoming a second language.
    ///
    /// **One verb per variant, and only where the variant has one answer to
    /// give.** `Effect::SearchLibrary { filter, finds, optional }` has three
    /// fields and two of them are real choices ("you may", and whether what
    /// is found goes to hand or battlefield), so it stays a literal — a
    /// `search` / `may_search` / `search_to_hand` family is how a vocabulary
    /// turns into a phrasebook. Thirty-four cards write it out and that is
    /// the right number.
    ///
    /// **The name is the word this pool already says**, which is usually the
    /// printed one. "Draw", "scry", "destroy", "exile" are all oracle text.
    /// "Blink" is what the engine had already named a thing oracle spells
    /// out in a clause ("exile it, then return it to the battlefield"), and
    /// it is two verbs, [`Effect::blink_to_owner`] and
    /// [`Effect::blink_to_you`], because the clause ends in one of two
    /// controllers and the card names which. [`Effect::bounce`] is the same
    /// shape from the other direction: the printing says "return … to its
    /// owner's hand" and the variant says `ReturnToHand`, but the table says
    /// bounce, and so did this repository before there was a verb —
    /// Cyclonic Rift's own comment calls both of its modes a bounce and
    /// Aether Channeler's effect list is named `BOUNCE_EFFECTS`. That is the
    /// owner's decision and it is paid for: `bounce` is a word no
    /// `//! Oracle:` header carries, so a grep from the printed sentence to
    /// the code stops here and at `blink_*`.
    ///
    /// It buys nothing where the variant is not one answer.
    /// [`Effect::ReturnAllToHand`] is the overloaded half of the same card
    /// and takes a filter *and* an `opponents_only` flag, so it stays a
    /// literal under the rule above — one use, two real choices.
    ///
    /// A fixed count is the argument, because 83 of the pool's 84 draws are
    /// fixed; the one that is not (and anything with `{X}`) writes the
    /// literal, exactly as [`Effect::mana_dynamic`] sits beside
    /// [`Effect::mana`].
    #[must_use]
    pub const fn draw(cards: u32) -> Self {
        Self::DrawCards {
            amount: Amount::Fixed(cards),
        }
    }

    /// "Scry 2."
    #[must_use]
    pub const fn scry(cards: u32) -> Self {
        Self::Scry {
            amount: Amount::Fixed(cards),
        }
    }

    /// "Surveil 1."
    #[must_use]
    pub const fn surveil(cards: u32) -> Self {
        Self::Surveil {
            amount: Amount::Fixed(cards),
        }
    }

    /// "You gain 3 life."
    #[must_use]
    pub const fn gain_life(life: u32) -> Self {
        Self::GainLife {
            amount: Amount::Fixed(life),
        }
    }

    /// "Return target … from your graveyard to the battlefield", the
    /// ordinary reanimation sentence: under **your** control and with
    /// nothing on it.
    #[must_use]
    pub const fn reanimate(target: TargetSpec) -> Self {
        Self::GraveyardToBattlefield {
            target,
            owner_control: false,
            counters: None,
        }
    }

    /// "…return it to the battlefield under its owner's control with a
    /// `n` `kind` counter on it" — the sentence undying and persist are
    /// (CR 702.93a, CR 702.79a).
    #[must_use]
    pub const fn return_to_owner_with(target: TargetSpec, kind: CounterKind, n: u16) -> Self {
        Self::GraveyardToBattlefield {
            target,
            owner_control: true,
            counters: Some((kind, n)),
        }
    }

    /// "Destroy target …"
    #[must_use]
    pub const fn destroy(target: TargetSpec) -> Self {
        Self::Destroy {
            target,
            no_regen: false,
        }
    }

    /// "Destroy target … It can't be regenerated." (CR 701.19c)
    #[must_use]
    pub const fn destroy_no_regen(target: TargetSpec) -> Self {
        Self::Destroy {
            target,
            no_regen: true,
        }
    }

    /// "Regenerate target …" / "Regenerate this creature."
    #[must_use]
    pub const fn regenerate(target: TargetSpec) -> Self {
        Self::Regenerate { target }
    }

    /// "~ deals N damage to each …" (Surtland Frostpyre, Dragonback Assault).
    /// A counted or `{X}` amount is the literal [`Self::DealDamageEach`].
    #[must_use]
    pub const fn damage_each(amount: u32, filter: &'static Filter) -> Self {
        Self::DealDamageEach {
            amount: Amount::Fixed(amount),
            filter,
        }
    }

    /// "Destroy all …" — a wrath the survivors may regenerate from.
    #[must_use]
    pub const fn destroy_all(filter: &'static Filter) -> Self {
        Self::DestroyAll {
            filter,
            no_regen: false,
        }
    }

    /// "Destroy all … They can't be regenerated." (CR 701.19c)
    #[must_use]
    pub const fn destroy_all_no_regen(filter: &'static Filter) -> Self {
        Self::DestroyAll {
            filter,
            no_regen: true,
        }
    }

    /// "Exile target …"
    #[must_use]
    pub const fn exile(target: TargetSpec) -> Self {
        Self::Exile { target }
    }

    /// "Exile target …" by an ability that another ability of the same
    /// object reads back as the card "exiled with" it (CR 607.2a), with no
    /// end of its own (Skyclave Apparition, Safe Haven).
    #[must_use]
    pub const fn exile_linked(target: TargetSpec) -> Self {
        Self::ExileLinked {
            target,
            until: None,
        }
    }

    /// "Exile target … until …" (CR 610.3): Werefox Bodyguard's "until this
    /// creature leaves the battlefield", Palace Jailer's "until an opponent
    /// becomes the monarch".
    #[must_use]
    pub const fn exile_until(target: TargetSpec, until: ExileUntil) -> Self {
        Self::ExileLinked {
            target,
            until: Some(until),
        }
    }

    /// "Exile target …, then return it to the battlefield under its owner's
    /// control" (Ephemerate).
    #[must_use]
    pub const fn blink_to_owner(target: TargetSpec) -> Self {
        Self::Blink {
            target,
            owner_control: true,
        }
    }

    /// "Exile target …, then return that card to the battlefield under your
    /// control" (Restoration Angel): the new object enters under the control
    /// of whoever controls the resolving spell or ability (CR 110.2a), and
    /// its owner stays who it was.
    #[must_use]
    pub const fn blink_to_you(target: TargetSpec) -> Self {
        Self::Blink {
            target,
            owner_control: false,
        }
    }

    /// "Return target … to its owner's hand."
    #[must_use]
    pub const fn bounce(target: TargetSpec) -> Self {
        Self::ReturnToHand { target }
    }

    /// A continuous effect this resolution creates, on the layer its
    /// modifier belongs to (CR 613.1).
    ///
    /// The layer is derived by [`crate::Modifier::layer`], for the reason
    /// [`crate::static_ability!`] gives: it is a function of the modifier
    /// and never a decision the card makes. Seventy-nine effects in the pool
    /// restated it, which is seventy-nine chances to write the wrong one —
    /// and `baylee_cards::lints` sweeps every last one of them.
    ///
    /// `filter` is `&Filter::This` for "the target", which is how a
    /// continuous effect says it; a filter naming a *kind* of object here is
    /// the Karn bug, and there is a lint for that too.
    #[must_use]
    pub const fn continuous(
        filter: &'static Filter,
        modifier: crate::static_ability::Modifier,
        duration: crate::static_ability::Duration,
    ) -> Self {
        Self::CreateContinuousEffect {
            layer: modifier.layer(),
            filter,
            modifier,
            duration,
        }
    }

    /// `Add {G}` / `Add {C}{C}` — a fixed amount of one named color.
    ///
    /// The unified [`Effect::AddMana`] answers three questions at once, and
    /// spelling all three out for the commonest line on a card would be a
    /// step backwards from the variant it replaced. These constructors are
    /// what card files use.
    #[must_use]
    pub const fn mana(color: ManaColor, amount: u32) -> Self {
        Self::AddMana {
            source: ManaSource::Fixed(color),
            amount: Amount::Fixed(amount),
            combination: false,
            restriction: None,
        }
    }

    /// `Add {G} or {U}.` — one mana, colour chosen on resolution.
    #[must_use]
    pub const fn mana_choice(colors: &'static [ManaColor]) -> Self {
        Self::AddMana {
            source: ManaSource::Choice(colors),
            amount: Amount::Fixed(1),
            combination: false,
            restriction: None,
        }
    }

    /// `Add one mana of any color.`
    #[must_use]
    pub const fn mana_of_any_color() -> Self {
        Self::mana_choice(crate::ALL_MANA_COLORS)
    }

    /// `Add one mana of the chosen color.` — the colour named as this
    /// permanent entered, through `EnterModifier::ChooseColor`.
    #[must_use]
    pub const fn mana_chosen() -> Self {
        Self::AddMana {
            source: ManaSource::Chosen,
            amount: Amount::Fixed(1),
            combination: false,
            restriction: None,
        }
    }

    /// `Add {W} or one mana of the chosen color.`
    #[must_use]
    pub const fn mana_chosen_or(colors: &'static [ManaColor]) -> Self {
        Self::AddMana {
            source: ManaSource::ChosenOr(colors),
            amount: Amount::Fixed(1),
            combination: false,
            restriction: None,
        }
    }

    /// `Add one mana of any color in your commander's color identity.`
    #[must_use]
    pub const fn mana_commander_identity() -> Self {
        Self::AddMana {
            source: ManaSource::CommanderIdentity,
            amount: Amount::Fixed(1),
            combination: false,
            restriction: None,
        }
    }

    /// `Add {G} for each Ally you control` — one color, counted amount.
    #[must_use]
    pub const fn mana_dynamic(color: ManaColor, amount: Amount) -> Self {
        Self::AddMana {
            source: ManaSource::Fixed(color),
            amount,
            combination: false,
            restriction: None,
        }
    }

    /// `Add X mana of any one color, where X is …` — one pick, counted
    /// amount.
    ///
    /// The fourth corner of the two questions the others answer between them,
    /// and it was the missing one: [`Self::mana_choice`] is a pick of one
    /// mana, [`Self::mana_dynamic`] is a counted amount of a named colour,
    /// and [`Self::mana_combination`] is a counted amount with a pick *each*.
    /// Harabaz Druid prints "any **one** color" and was written with the last
    /// of those, which the engine reads as X colour prompts (`resolve::mana`:
    /// `let (picks, per_pick) = if combination { (n, 1) } else { (1, n) };`).
    /// Widening `mana_combination` would not have fixed it, because "in any
    /// combination" is a real and different sentence — this is the shape that
    /// was not sayable.
    #[must_use]
    pub const fn mana_choice_dynamic(colors: &'static [ManaColor], amount: Amount) -> Self {
        Self::AddMana {
            source: ManaSource::Choice(colors),
            amount,
            combination: false,
            restriction: None,
        }
    }

    /// `Add {W}{U} in any combination of colors.` — one pick per mana,
    /// which is what "in any combination" means and what a single choice
    /// for the whole amount does not.
    #[must_use]
    pub const fn mana_combination(colors: &'static [ManaColor], amount: Amount) -> Self {
        Self::AddMana {
            source: ManaSource::Choice(colors),
            amount,
            combination: true,
            restriction: None,
        }
    }

    /// `Add one mana of any color that a land an opponent controls could
    /// produce` (Exotic Orchard, Fellwar Stone), or you control. A colour,
    /// so never colorless (CR 106.1a).
    #[must_use]
    pub const fn mana_land_color(mine: bool) -> Self {
        Self::AddMana {
            source: ManaSource::LandColor {
                mine,
                any_type: false,
            },
            amount: Amount::Fixed(1),
            combination: false,
            restriction: None,
        }
    }

    /// `Add one mana of any type that a land you control could produce`
    /// (Reflecting Pool), or an opponent's. A type, so colorless too
    /// (CR 106.1b).
    #[must_use]
    pub const fn mana_land_type(mine: bool) -> Self {
        Self::AddMana {
            source: ManaSource::LandColor {
                mine,
                any_type: true,
            },
            amount: Amount::Fixed(1),
            combination: false,
            restriction: None,
        }
    }

    /// `Spend this mana only to cast …` — the tail of a mana line, written
    /// where the card writes it (Cavern of Souls).
    ///
    /// # Panics
    /// At compile time, when applied to anything but a mana effect.
    #[must_use]
    pub const fn restricted(self, filter: &'static Filter, rider: SpendRider) -> Self {
        let Self::AddMana {
            source,
            amount,
            combination,
            ..
        } = self
        else {
            panic!("restricted() describes mana, and only Effect::AddMana produces it")
        };
        Self::AddMana {
            source,
            amount,
            combination,
            restriction: Some(ManaRestriction {
                filter,
                rider,
                restricts: true,
                until_end_of_turn: false,
            }),
        }
    }

    /// `When that mana is spent to cast …` / `If that mana is spent on …` —
    /// a rider that restricts nothing (Path of Ancestry, Boseiju, Who
    /// Shelters All). The mana pays for anything; spent on a spell `filter`
    /// matches, it sets the rider off, once for each unit (CR 106.6a).
    ///
    /// # Panics
    /// At compile time, when applied to anything but a mana effect, or with
    /// [`SpendRider::None`]: a rider that does nothing says nothing.
    #[must_use]
    pub const fn when_spent(self, filter: &'static Filter, rider: SpendRider) -> Self {
        assert!(
            !matches!(rider, SpendRider::None),
            "when_spent() names what spending the mana does, and SpendRider::None does nothing"
        );
        let Self::AddMana {
            source,
            amount,
            combination,
            ..
        } = self
        else {
            panic!("when_spent() describes mana, and only Effect::AddMana produces it")
        };
        Self::AddMana {
            source,
            amount,
            combination,
            restriction: Some(ManaRestriction {
                filter,
                rider,
                restricts: false,
                until_end_of_turn: false,
            }),
        }
    }
    /// The effect lists this one runs, as at most two slices.
    ///
    /// **Ten variants carry another effect and this is the only list of
    /// them.** Before it existed there were five: two pool lints, three
    /// readers in `baylee-ai`, and a coverage probe, each with its own
    /// match and each with different holes. Every one of them descended
    /// into `Sequence` and `MayDo`; none of them descended into
    /// [`Effect::PlayerMayPayOr`] or [`Effect::PlayerMayPayCostOr`], so
    /// whatever a card hid behind "unless you pay" was invisible to all
    /// five — 35 effects in the pool as this was written — and three of the
    /// AI readers also stopped short of the five conditionals. A lint blind
    /// inside that clause reports a clean pool it never read, which is worse
    /// than a lint that fails.
    ///
    /// Two slices is the shape because [`Effect::IfKicked`] is the widest
    /// carrier, and it allocates nothing: an agent asks this per decision.
    ///
    /// The match is **exhaustive with no wildcard**, which is the whole
    /// mechanism. An eleventh carrier added to [`Effect`] does not quietly
    /// fall into a `_` and go unwalked by everything at once; it fails to
    /// compile here, in the crate that owns the enum, and the person adding
    /// it says where its branches are.
    #[must_use]
    #[allow(clippy::too_many_lines)] // every variant named once; the length is the guarantee
    pub fn branches(&self) -> (&'static [Effect], &'static [Effect]) {
        const NONE: &[Effect] = &[];
        match self {
            // **Every carrier below binds every one of its fields**, with
            // no `..` in any of these ten arms. That is the second half of
            // the guarantee, and it is deliberate: an exhaustive match over
            // *variants* says nothing about a new *field* on one of them, so
            // `IfKicked { then, .. }` would let a third branch be added and
            // go unwalked by everything at once — the very defect this
            // method closes. Fields the walk does not use are bound to `_`
            // by name, so adding one is a build error here and a decision
            // somebody makes on purpose.
            //
            // The non-carrying variants with fields do use `..`, and that
            // is the one hole left: giving `GainLife` an effect field would
            // be caught by nobody. It is left open on the measurement that such a
            // field makes the variant a carrier, which is a thing one writes
            // rather than stumbles into — where a *twelfth* branch on a
            // variant that already nests is exactly what somebody stumbles
            // into, because the arm already looks handled.
            // A reflexive ability's body is its own stack object later, but
            // it is still what this list can come to do, and every walker
            // has to read it.
            Effect::Sequence(effects)
            | Effect::MayDo { effects }
            | Effect::MayDoOnceEachTurn { effects }
            | Effect::NthResolutionThisTurn { effects }
            | Effect::Reflexive {
                when: _,
                effects,
                target: _,
            }
            // What the payment buys: the list runs on a yes.
            | Effect::PlayerMayPayThen {
                player: _,
                mana: _,
                effects,
            } => (effects, NONE),
            Effect::IfCreaturesDiedAtLeast { n: _, then }
            | Effect::ChooseYoursThen { filter: _, then }
            | Effect::IfTargetMatches { filter: _, then }
            | Effect::IfNoCountersOnSelf { kind: _, then }
            | Effect::IfNotLostLifeThisTurn { then }
            | Effect::IfResolvedTimesThisTurn { times: _, then }
            | Effect::IfControlGreatestCmc { filter: _, then } => (then, NONE),
            Effect::IfKicked { then, otherwise }
            | Effect::IfCondition {
                condition: _,
                then,
                otherwise,
            }
            | Effect::IfEventPowerAtLeast {
                n: _,
                then,
                otherwise,
            } => (then, otherwise),
            // A single effect, not a list: the branch taken when the player
            // declines the price. `from_ref` is what makes it one shape with
            // the rest rather than a second kind of caller.
            Effect::PlayerMayPayOr {
                player: _,
                mana: _,
                effect,
            }
            | Effect::PlayerMayPayLifeOr { effect, .. }
            | Effect::PlayerMayPayCostOr {
                player: _,
                cost: _,
                effect,
            } => (core::slice::from_ref(*effect), NONE),
            Effect::GainLife { .. }
            | Effect::GainLifeFor { .. }
            | Effect::Exile { .. }
            | Effect::Blink { .. }
            | Effect::LookAtTopPick { .. }
            | Effect::LookAtTopKeepBottomPlay { .. }
            | Effect::ChooseExiledToPlay { .. }
            | Effect::PayLifeOrPutBackDrawn { .. }
            | Effect::RevealTopAndSort { .. }
            | Effect::RevealUntil { .. }
            | Effect::Cascade
            | Effect::MayCastTarget { .. }
            | Effect::MillMayTakeOne { .. }
            | Effect::RevealAndSeparate { .. }
            | Effect::LookAtTopMayPut { .. }
            | Effect::DiscardUpToThenDraw { .. }
            | Effect::SearchLibraryOrGraveyard { .. }
            | Effect::PutFromHandOnTop { .. }
            | Effect::PutFromHandOntoBattlefield { .. }
            | Effect::LoseLife { .. }
            | Effect::DrawCards { .. }
            | Effect::DrawCardsFor { .. }
            | Effect::ExileTargetsCreateTokens { .. }
            | Effect::DealDamage { .. }
            | Effect::Fight { .. }
            | Effect::DamageEqualToPower { .. }
            | Effect::EventObjectDealsDamageEqualToPower { .. }
            | Effect::DealDamageToTargetController { .. }
            | Effect::DealDamageEach { .. }
            | Effect::PreventNextDamage { .. }
            | Effect::PreventAllCombatDamageThisTurn
            | Effect::WishToHand { .. }
            | Effect::Destroy { .. }
            | Effect::PutTargetOnBottomOfLibrary
            | Effect::PutOnBottomOfLibraryFromGraveyard { .. }
            | Effect::GrantFlashback
            | Effect::TakeExtraTurn
            | Effect::ExileSource
            | Effect::TapTarget
            | Effect::TapAll { .. }
            | Effect::ExileTopMayCast { .. }
            | Effect::UntapTarget
            | Effect::UntapSelf
            | Effect::ExileAndReturnAtEndStep
            | Effect::OwnerPutsOnTopOrBottom { .. }
            | Effect::ExileIfDiesThisTurn { .. }
            | Effect::Discover { .. }
            | Effect::RevealTopOnePerType { .. }
            | Effect::DealDamageDivided { .. }
            | Effect::CounterTargetSpellToExile
            | Effect::CounterTargetSpell
            | Effect::CounterTargetAbility
            | Effect::CounterTargetSpellOrAbility
            | Effect::TargetSourceLosesAbilities { .. }
            | Effect::DelayedManaAtNextFirstMain { .. }
            | Effect::ChangeTarget { .. }
            | Effect::ChooseNewTargets
            | Effect::ExchangeControlOrSacrifice
            | Effect::ExchangeControl
            | Effect::DestroyChosenForPlayers { .. }
            | Effect::DiscardForPlayers { .. }
            | Effect::DiscardRandom { .. }
            | Effect::RevealHandDiscard { .. }
            | Effect::AllGraveyardCreaturesToBattlefield
            | Effect::GraveyardAllToHand { .. }
            | Effect::YourGraveyardToBattlefield { .. }
            | Effect::Earthbend(_)
            | Effect::ReturnToBattlefieldTapped { .. }
            | Effect::TransformSource
            | Effect::TransformSourceAtNextUpkeep
            | Effect::ExileSelfReturnAsFace { .. }
            | Effect::SacrificeFilter { .. }
            | Effect::ReturnChosenToHand { .. }
            | Effect::UntapChosen { .. }
            | Effect::DrainAllCountersIntoSelf
            | Effect::ShuffleGraveyardIntoLibrary
            | Effect::BecomePrepared
            | Effect::GainLifeDoubleX
            | Effect::SearchLibrary { .. }
            | Effect::Scry { .. }
            | Effect::Surveil { .. }
            | Effect::ScryFor { .. }
            | Effect::ExileLibraryAndShuffleHand { .. }
            | Effect::SetPTFilter { .. }
            | Effect::Mill { .. }
            | Effect::AddMana { .. }
            | Effect::GrantSubtype { .. }
            | Effect::AddCounter { .. }
            | Effect::AddCounterFilter { .. }
            | Effect::DoubleCountersFilter { .. }
            | Effect::ReturnToHand { .. }
            | Effect::ReturnAllToHand { .. }
            | Effect::DestroyAll { .. }
            | Effect::ExileAll { .. }
            | Effect::DestroyOthersNamedLike { .. }
            | Effect::ExileGraveyard { .. }
            | Effect::GraveyardToTop { .. }
            | Effect::GraveyardToHand { .. }
            | Effect::GraveyardToBattlefield { .. }
            | Effect::CreateTokenPtPerCount { .. }
            | Effect::CreateToken { .. }
            | Effect::CreateTokenN { .. }
            | Effect::CreateTokenForTargetController { .. }
            | Effect::Amass { .. }
            | Effect::PutSourceOnTopOfLibrary
            | Effect::CreateTokenCopyOf { .. }
            | Effect::Populate
            | Effect::CreateTokenCopyOfEquipped { .. }
            | Effect::CreateTokenCopyOfTarget { .. }
            | Effect::CreateTokenCopyOfSource { .. }
            | Effect::CreateTokenCopyOfFirstToken
            | Effect::BottomCardFromHand { .. }
            | Effect::CopyTargetSpell { .. }
            | Effect::CopyTargetAbility
            | Effect::CopyThisSpell
            | Effect::AttachSelf { .. }
            | Effect::ReorderTopLibrary { .. }
            | Effect::PayLifeOrEnterTapped { .. }
            | Effect::CreateContinuousEffect { .. }
            | Effect::ChangeController { .. }
            | Effect::AllCreaturesToOwner
            | Effect::ControlRotation
            | Effect::PhaseOut { .. }
            | Effect::ExileLinked { .. }
            | Effect::ExileTargetsWithSource
            | Effect::ReturnLinkedToBattlefield
            | Effect::CreateTokenFromLinked { .. }
            | Effect::SacrificeSelf
            | Effect::PayCostOrLoseLater { .. }
            | Effect::CreateEmblem { .. }
            | Effect::BecomeMonarch(_)
            | Effect::OptionalBasicLandSearchFor { .. }
            | Effect::SearchLibraryOf { .. }
            | Effect::SearchLibraryUpTo { .. }
            | Effect::SearchOpponentSplits { .. }
            | Effect::PumpFilter { .. }
            | Effect::ProtectionFromChosenColor { .. }
            | Effect::Regenerate { .. }
            | Effect::PumpTarget { .. } => (NONE, NONE),
        }
    }

    /// Every effect reachable from this one, itself included, depth first.
    ///
    /// The walk [`Self::branches`] exists for. `seen` is what a caller holds
    /// against a floor: a probe reporting nought over a pool is only news
    /// once it says how much it read to get there.
    pub fn walk(
        effects: &'static [Effect],
        seen: &mut usize,
        visit: &mut impl FnMut(&'static Effect),
    ) {
        for effect in effects {
            *seen += 1;
            visit(effect);
            let (then, otherwise) = effect.branches();
            Self::walk(then, seen, visit);
            Self::walk(otherwise, seen, visit);
        }
    }
}

#[cfg(test)]
mod verb_tests {
    use super::*;

    #[test]
    fn life_payment_fallback_is_visited() {
        static EFFECTS: &[Effect] = &[Effect::PlayerMayPayLifeOr {
            player: PlayerRel::ControllerOfTarget,
            life: Amount::SourcePower,
            effect: &Effect::CounterTargetSpellOrAbility,
        }];
        let mut seen = 0;
        let mut counter_seen = false;
        Effect::walk(EFFECTS, &mut seen, &mut |effect| {
            counter_seen |= matches!(effect, Effect::CounterTargetSpellOrAbility);
        });
        assert_eq!(seen, 2);
        assert!(counter_seen);
    }

    /// Crystal Rod's life gain sits behind the payment, and a pool walk has
    /// to find it there.
    #[test]
    fn a_price_paid_body_is_visited() {
        static EFFECTS: &[Effect] = &[Effect::PlayerMayPayThen {
            player: PlayerRel::You,
            mana: Amount::Fixed(1),
            effects: &[Effect::GainLife {
                amount: Amount::Fixed(1),
            }],
        }];
        let mut seen = 0;
        let mut gain_seen = false;
        Effect::walk(EFFECTS, &mut seen, &mut |effect| {
            gain_seen |= matches!(effect, Effect::GainLife { .. });
        });
        assert_eq!(seen, 2);
        assert!(gain_seen);
    }

    /// Nissa, Resurgent Animist's reveal sits inside
    /// `IfResolvedTimesThisTurn`, and a pool walk has to find it there.
    #[test]
    fn the_nth_resolution_branch_is_visited() {
        static EFFECTS: &[Effect] = &[Effect::IfResolvedTimesThisTurn {
            times: 2,
            then: &[Effect::RevealUntil {
                filter: &crate::Filter::Any,
                found: SearchDest::Hand,
            }],
        }];
        let mut seen = 0;
        let mut reveal_seen = false;
        Effect::walk(EFFECTS, &mut seen, &mut |effect| {
            reveal_seen |= matches!(effect, Effect::RevealUntil { .. });
        });
        assert_eq!(seen, 2);
        assert!(reveal_seen);
    }

    /// Each of the nth-resolution effects is walked.
    #[test]
    fn nth_resolution_effects_are_visited() {
        static EFFECTS: &[Effect] = &[Effect::NthResolutionThisTurn {
            effects: &[Effect::gain_life(4), Effect::draw(1)],
        }];
        let mut seen = 0;
        let mut draw_seen = false;
        Effect::walk(EFFECTS, &mut seen, &mut |effect| {
            draw_seen |= matches!(effect, Effect::DrawCards { .. });
        });
        assert_eq!(seen, 3);
        assert!(draw_seen);
    }

    /// "You may …. Do this only once each turn." carries its body the way
    /// `MayDo` does, and the walk goes into it.
    #[test]
    fn once_each_turn_body_is_visited() {
        static EFFECTS: &[Effect] = &[Effect::MayDoOnceEachTurn {
            effects: &[Effect::GraveyardToBattlefield {
                target: TargetSpec::EventObject,
                owner_control: false,
                counters: None,
            }],
        }];
        let mut seen = 0;
        let mut body_seen = false;
        Effect::walk(EFFECTS, &mut seen, &mut |effect| {
            body_seen |= matches!(effect, Effect::GraveyardToBattlefield { .. });
        });
        assert_eq!(seen, 2);
        assert!(body_seen);
    }

    /// "Choose a creature you control. It …" carries what happens to the
    /// chosen one the way a one-branch conditional does, and the walk goes
    /// into it.
    #[test]
    fn chosen_permanent_body_is_visited() {
        static EFFECTS: &[Effect] = &[Effect::ChooseYoursThen {
            filter: &crate::Filter::CREATURE,
            then: &[Effect::draw(1)],
        }];
        let mut seen = 0;
        let mut body_seen = false;
        Effect::walk(EFFECTS, &mut seen, &mut |effect| {
            body_seen |= matches!(effect, Effect::DrawCards { .. });
        });
        assert_eq!(seen, 2);
        assert!(body_seen);
    }

    /// "… if it's [filter]" carries its effects the way the other one-branch
    /// conditionals do, and the walk goes into them.
    #[test]
    fn if_target_matches_body_is_visited() {
        static EFFECTS: &[Effect] = &[Effect::IfTargetMatches {
            filter: &crate::Filter::CmcAtMostColorsSpent,
            then: &[Effect::Exile {
                target: TargetSpec::Object(&crate::Filter::NONLAND),
            }],
        }];
        let mut seen = 0;
        let mut body_seen = false;
        Effect::walk(EFFECTS, &mut seen, &mut |effect| {
            body_seen |= matches!(effect, Effect::Exile { .. });
        });
        assert_eq!(seen, 2);
        assert!(body_seen);
    }

    use crate::ability::Trigger;
    use crate::static_ability::{Duration, Layer, Modifier};

    /// Every verb is the literal it replaces.
    ///
    /// A verb is only worth having if adopting it is free, and "free" here
    /// means the same `Effect` value and not a near-enough one. There was no
    /// test of this shape for `Effect::mana` either, which has 219 uses.
    #[test]
    fn a_verb_is_the_literal_it_replaces() {
        assert_eq!(
            Effect::draw(3),
            Effect::DrawCards {
                amount: Amount::Fixed(3)
            }
        );
        assert_eq!(
            Effect::scry(2),
            Effect::Scry {
                amount: Amount::Fixed(2)
            }
        );
        assert_eq!(
            Effect::surveil(1),
            Effect::Surveil {
                amount: Amount::Fixed(1)
            }
        );
        assert_eq!(
            Effect::gain_life(4),
            Effect::GainLife {
                amount: Amount::Fixed(4)
            }
        );

        let target = TargetSpec::Object(&Filter::CREATURE);
        assert_eq!(
            Effect::destroy(target),
            Effect::Destroy {
                target,
                no_regen: false
            }
        );
        assert_eq!(Effect::exile(target), Effect::Exile { target });
        assert_eq!(
            Effect::blink_to_owner(target),
            Effect::Blink {
                target,
                owner_control: true
            }
        );
        assert_eq!(
            Effect::blink_to_you(target),
            Effect::Blink {
                target,
                owner_control: false
            }
        );
        assert_eq!(Effect::bounce(target), Effect::ReturnToHand { target });
    }

    /// `Effect::continuous` derives the layer, and derives the one the
    /// seventy-nine effects in the pool already state.
    #[test]
    fn a_continuous_effect_derives_its_own_layer() {
        assert_eq!(
            Effect::continuous(
                &Filter::This,
                Modifier::ModifyPT(1, 1),
                Duration::UntilEndOfTurn,
            ),
            Effect::CreateContinuousEffect {
                layer: Layer::PtModify,
                filter: &Filter::This,
                modifier: Modifier::ModifyPT(1, 1),
                duration: Duration::UntilEndOfTurn,
            }
        );
        let Effect::CreateContinuousEffect { layer, .. } = Effect::continuous(
            &Filter::Any,
            Modifier::AddType(baylee_core::types::TypeSet::ARTIFACT),
            Duration::WhileSourceOnBattlefield,
        ) else {
            panic!("continuous must build CreateContinuousEffect");
        };
        assert_eq!(layer, Layer::Type, "a type change is layer 4 (CR 613.1)");
    }

    /// `Trigger::ETB` is the enter-trigger 99 of the pool's 110 spell out.
    #[test]
    fn etb_is_the_trigger_the_pool_writes_a_hundred_times() {
        assert_eq!(Trigger::ETB, Trigger::EntersBattlefield(&Filter::This));
    }
}

#[cfg(test)]
mod amount_and_target_tests {
    use super::*;

    /// Which way an amount counts, asked of every variant there is.
    ///
    /// This is the one place the question is answered, because it was four:
    /// three closures in `resolve::counters` and one arm in the AI's tactics
    /// each spelled `matches!(a, Amount::NegX | Amount::NegXFixed(_))` — a
    /// positive list over an enum, four times over, and four chances for the
    /// next negative amount to be read as a bonus. Nothing in a suite reads
    /// a sign as a bug on its own: the card resolves, the creature changes
    /// size, and only the direction is wrong.
    ///
    /// All fifteen variants are named. That is a population rather than a
    /// guard — the exhaustive `match` is what a new variant has to answer —
    /// but the answers themselves are what no compiler can check.
    #[test]
    fn every_amount_says_which_way_it_counts() {
        const NEG: &Amount = &Amount::NegX;
        const POS: &Amount = &Amount::X;
        let downwards: Vec<Amount> = vec![Amount::NegX, Amount::NegXFixed(3), Amount::Negated(POS)];
        let upwards: Vec<Amount> = vec![
            Amount::Fixed(2),
            Amount::X,
            Amount::XPlusCommanderCasts,
            Amount::DoubleX,
            Amount::DistinctColorsAmong(&Filter::CREATURE),
            Amount::TargetPower,
            Amount::SourcePower,
            Amount::CountersOnSource(CounterKind::Charge),
            Amount::TargetCmc,
            Amount::SacrificedManaValue,
            Amount::ManaSpentToCast,
            Amount::CountOf {
                filter: &Filter::CREATURE,
                zone: ZoneSel::Battlefield,
            },
            // A negated negative: parity rather than a refusal, because that
            // is what the word means. No card prints a double negative, and
            // the rule is cheaper than a refusal that has to be remembered.
            Amount::Negated(NEG),
        ];
        assert_eq!(
            downwards.len() + upwards.len(),
            16,
            "sixteen values over fifteen variants — `Negated` is in both \
             lists, which is the parity rule being read from both sides"
        );
        for a in downwards {
            assert!(a.is_negative(), "{a:?} counts downwards");
        }
        for a in upwards {
            assert!(!a.is_negative(), "{a:?} does not count downwards");
        }
    }

    /// "Up to one target" is not one target, and reading it as one is an
    /// ability a player cannot activate at all on an empty board — for
    /// Teferi, Time Raveler a card that cannot be drawn, and for Karn, the
    /// Great Creator a loyalty tick that cannot be taken. The four
    /// constructors are the whole vocabulary a card has for saying how many.
    #[test]
    fn how_many_targets_a_card_asks_for() {
        const WHAT: TargetSpec = TargetSpec::Object(&Filter::CREATURE);

        let one = TargetReq::one(WHAT);
        assert_eq!((one.min, one.max), (1, 1));
        assert!(!one.count_is_x);

        let maybe = TargetReq::up_to_one(WHAT);
        assert_eq!(
            (maybe.min, maybe.max),
            (0, 1),
            "a minimum of nought is the whole of what 'up to' means"
        );
        assert_ne!(
            maybe, one,
            "the two are different requirements and not a spelling"
        );

        let two = TargetReq::up_to(WHAT, 2);
        assert_eq!((two.min, two.max), (0, 2));
        assert_eq!(
            TargetReq::up_to(WHAT, 1),
            maybe,
            "'up to one' is 'up to' with a one in it"
        );

        let x = TargetReq::x_targets(WHAT);
        assert_eq!(
            (x.min, x.max, x.count_is_x),
            (0, 255, true),
            "an X count is unbounded until X is announced"
        );
        let pair = TargetReq::exactly(WHAT, 2);
        assert_eq!((pair.min, pair.max), (2, 2), "'two target lands' is two");
        assert_eq!(
            TargetReq::exactly(WHAT, 1),
            one,
            "'one' is 'exactly' with a one in it"
        );

        for req in [one, maybe, two, x, pair] {
            assert_eq!(req.spec, WHAT, "each of them targets what it was given");
        }
    }

    /// `bounds` is the one reading of "how many" a cast and an activation
    /// share: a fixed count is its own bounds, and an X count is X once X has
    /// been announced.
    #[test]
    fn bounds_read_x_only_where_the_count_is_x() {
        const WHAT: TargetSpec = TargetSpec::Object(&Filter::CREATURE);
        assert_eq!(
            TargetReq::up_to(WHAT, 2).bounds(7),
            (0, 2),
            "X is not this count"
        );
        assert_eq!(TargetReq::exactly(WHAT, 2).bounds(0), (2, 2));
        let x = TargetReq::x_targets(WHAT);
        assert_eq!(x.bounds(0), (0, 0), "X = 0 targets nothing");
        assert_eq!(x.bounds(3), (3, 3), "X targets is exactly X");
        assert_eq!(
            x.bounds(300),
            (255, 255),
            "a count past the field's width saturates rather than wrapping to 44"
        );
    }
}
