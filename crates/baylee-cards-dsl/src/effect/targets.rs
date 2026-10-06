//! What an effect points at: zones, players, target specs and the slots a
//! spell's targets fill.

use super::Filter;

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
    /// The hand of the player whose turn it is.
    HandActivePlayer,
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
    /// The owner of the ability's source, including its last known incarnation.
    OwnerOfSource,
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
    /// A ranged printed maximum 255 means any number; exactly 255 remains
    /// exact. A pending choice
    /// bounds that open maximum by its actual legal options. X retains its
    /// full announced value; boards may contain more than 255 objects.
    #[must_use]
    pub const fn bounds(self, x: u32) -> (u32, u32) {
        if self.count_is_x {
            (x, x)
        } else {
            (
                self.min as u32,
                if self.max == u8::MAX && self.min < self.max {
                    u32::MAX
                } else {
                    self.max as u32
                },
            )
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
