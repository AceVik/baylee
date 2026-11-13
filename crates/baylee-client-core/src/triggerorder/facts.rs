//! What a trigger will do, read off the ability the card compiled to: the
//! same script the engine runs, so no translation and no printing can make
//! it say something else, and nothing here reads anybody's hidden cards.
//!
//! The reading is deliberately partial. It knows the verbs that decide an
//! order, says nothing about the rest, and a rule with nothing to go on
//! leaves two triggers as they came. `pool_tests` keeps count of how much of
//! the pool it reads, so that a new verb going unread shows up as a number.

use baylee_cards::dsl::{
    AbilityDef, Trigger,
    effect::{Amount, Effect, PlayerRel, TargetSpec},
};
use baylee_core::ids::PlayerId;

use super::TriggerKey;

/// A change to one player's totals that is certain, if the trigger resolves.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Change {
    /// Whose.
    pub who: PlayerId,
    /// What changes.
    pub what: Total,
}

/// What a [`Change`] does.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Total {
    /// Life gained or lost, damage included.
    Life(i32),
    /// Poison counters.
    Poison(u32),
    /// Cards drawn: a library drawn past its end loses the game.
    Draws(u32),
    /// The player loses the game.
    Loses,
    /// The player wins the game.
    Wins,
}

/// Kinds of thing a trigger does, as a set.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub struct Deeds(u16);

impl Deeds {
    /// Draws cards for the player.
    pub const DRAWS: Self = Self(1 << 0);
    /// Looks at or sorts the top of the library.
    pub const LOOKS: Self = Self(1 << 1);
    /// Shuffles the library.
    pub const SHUFFLES: Self = Self(1 << 2);
    /// Finds a card.
    pub const SEARCHES: Self = Self(1 << 3);
    /// Shows the player more: a card, a library's top, a search.
    pub const INFORMS: Self = Self(Self::DRAWS.0 | Self::LOOKS.0 | Self::SEARCHES.0);
    /// Asks the player something as it resolves.
    pub const ASKS: Self = Self(1 << 4);
    /// Takes something off the battlefield or out of a fight.
    pub const REMOVES: Self = Self(1 << 5);
    /// Makes a permanent bigger or better.
    pub const PUMPS: Self = Self(1 << 6);
    /// Takes its own source away.
    pub const LEAVES: Self = Self(1 << 7);
    /// Lets the player decline it as it resolves.
    pub const MAY: Self = Self(1 << 11);
    /// Costs an opponent life or poisons them.
    pub const HURTS_OPPONENT: Self = Self(1 << 8);
    /// Costs the player life.
    pub const HURTS_ME: Self = Self(1 << 9);
    /// Gains the player life.
    pub const HEALS_ME: Self = Self(1 << 10);

    /// Whether any of `other` is in this set.
    #[must_use]
    pub const fn has(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }

    const fn add(&mut self, other: Self) {
        self.0 |= other.0;
    }
}

/// Things one trigger can make and another can count, as a set.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub struct Stuff(u16);

impl Stuff {
    /// Counters on some other permanent.
    pub const COUNTERS: Self = Self(1 << 0);
    /// Permanents on the battlefield.
    pub const BOARD: Self = Self(1 << 1);
    /// Cards in a graveyard.
    pub const GRAVEYARD: Self = Self(1 << 2);
    /// Cards in hand.
    pub const HAND: Self = Self(1 << 3);
    /// The player's life total.
    pub const LIFE: Self = Self(1 << 4);
    /// Mana in the pool.
    pub const MANA: Self = Self(1 << 5);
    /// A creature's power.
    pub const POWER: Self = Self(1 << 6);
    /// What can be met across the table, whatever the sources.
    pub const ANYWHERE: Self = Self(
        Self::COUNTERS.0
            | Self::BOARD.0
            | Self::GRAVEYARD.0
            | Self::HAND.0
            | Self::LIFE.0
            | Self::MANA.0
            | Self::POWER.0,
    );
    /// Counters on the trigger's own source: met only by that source's others.
    pub const OWN_COUNTERS: Self = Self(1 << 7);

    /// Whether this makes something `reads` counts, within `scope`.
    #[must_use]
    pub const fn meets(self, reads: Self, scope: Self) -> bool {
        self.0 & reads.0 & scope.0 != 0
    }

    const fn add(&mut self, other: Self) {
        self.0 |= other.0;
    }
}

/// What a trigger will do.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Facts {
    /// What is certain to happen to players' totals, in the order it happens.
    pub changes: Vec<Change>,
    /// What kind of thing it does, certain or not.
    pub deeds: Deeds,
    /// What it brings into being, for a later trigger to count.
    pub makes: Stuff,
    /// What it counts as it resolves.
    pub reads: Stuff,
}

impl Facts {
    /// Whether this says anything at all.
    #[must_use]
    pub fn known(&self) -> bool {
        *self != Self::default()
    }

    /// Adds what these followers do when this sets them off, and what those
    /// set off in turn, so a trigger is placed for all it brings about. What
    /// a follower does is never certain: its filter may not match, so it adds
    /// no [`Change`], and the game's end is claimed without it.
    #[must_use]
    pub fn with_followers(mut self, followers: &[Follower]) -> Self {
        for _ in followers {
            let before = (self.deeds, self.makes);
            for follower in followers {
                if self.makes.meets(follower.wakes, Stuff::ANYWHERE) {
                    self.deeds.add(follower.facts.deeds);
                    self.makes.add(follower.facts.makes);
                }
            }
            if before == (self.deeds, self.makes) {
                break;
            }
        }
        self
    }
}

/// A trigger of a permanent on the battlefield that a waiting trigger would
/// set off: its effects go on the stack above whatever waits below, so where
/// the waiting trigger stands is where they happen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Follower {
    /// What sets it off.
    wakes: Stuff,
    /// What it then does.
    facts: Facts,
}

/// The follower this ability is, when something a trigger makes wakes it.
#[must_use]
pub fn follower(ability: &AbilityDef, reading: &Reading<'_>) -> Option<Follower> {
    let AbilityDef::Triggered { trigger, .. } = ability else {
        return None;
    };
    let wakes = match trigger {
        Trigger::EntersBattlefield(_) | Trigger::EntersBattlefieldFromGraveyardOrCast(_) => {
            Stuff::BOARD
        }
        Trigger::Draws(PlayerRel::You) => Stuff::HAND,
        Trigger::GainsLife(PlayerRel::You) => Stuff::LIFE,
        Trigger::CountersPlaced { .. } => Stuff::COUNTERS,
        _ => return None,
    };
    let facts = of_ability(ability, reading);
    facts.known().then_some(Follower { wakes, facts })
}

/// Who is who, for this one trigger.
#[derive(Clone, Copy, Debug)]
pub struct Reading<'a> {
    /// The trigger's controller, the player being asked.
    pub me: PlayerId,
    /// That player's opponents still in the game.
    pub opponents: &'a [PlayerId],
    /// The player its event names ("that land's controller").
    pub event_player: Option<PlayerId>,
    /// The amount its event carried ("that much").
    pub event_amount: Option<u32>,
}

/// Whom a damaging effect strikes, before any target is chosen: the question
/// comes first (CR 603.3b, 603.3d).
enum Struck {
    /// These players.
    Players(Vec<PlayerId>),
    /// A player or something else, whichever the target turns out to be.
    Open,
    /// A permanent.
    Object,
}

impl Reading<'_> {
    /// The players a relation names, when this trigger can say which.
    fn players(&self, relation: PlayerRel) -> Vec<PlayerId> {
        match relation {
            PlayerRel::You => vec![self.me],
            PlayerRel::Opponent => self.only_opponent(),
            PlayerRel::EachOpponent | PlayerRel::EachOtherPlayer => self.opponents.to_vec(),
            PlayerRel::EachPlayer => {
                let mut everyone = vec![self.me];
                everyone.extend_from_slice(self.opponents);
                everyone
            }
            PlayerRel::ControllerOfEvent => self.event_player.into_iter().collect(),
            _ => Vec::new(),
        }
    }

    /// The one opponent, where there is only one to mean.
    fn only_opponent(&self) -> Vec<PlayerId> {
        match self.opponents {
            [only] => vec![*only],
            _ => Vec::new(),
        }
    }

    /// Whom a target strikes.
    fn struck(&self, target: TargetSpec) -> Struck {
        match target {
            TargetSpec::Player(relation) => Struck::Players(self.players(relation)),
            TargetSpec::AnyOpponent => Struck::Players(self.only_opponent()),
            TargetSpec::AnyPlayer
            | TargetSpec::AnyTarget
            | TargetSpec::OpponentOrObject(_)
            | TargetSpec::PlayerOrObject(_) => Struck::Open,
            _ => Struck::Object,
        }
    }

    /// A number the trigger is sure of: printed, or the event's "that much".
    const fn fixed(&self, amount: Amount) -> Option<u32> {
        match amount {
            Amount::Fixed(value) => Some(value),
            Amount::EventAmount => self.event_amount,
            _ => None,
        }
    }
}

/// The compiled ability a key names.
#[must_use]
pub fn ability(key: TriggerKey) -> Option<&'static AbilityDef> {
    match key {
        TriggerKey::Card(ability) => baylee_cards::by_index(ability.card)?
            .abilities
            .get(usize::try_from(ability.index).ok()?),
        TriggerKey::Token { token, index } => baylee_cards::tokens::by_token_id(token)?
            .abilities
            .get(usize::try_from(index).ok()?),
    }
}

/// What this ability will do, for this player.
#[must_use]
pub fn of_ability(ability: &AbilityDef, reading: &Reading<'_>) -> Facts {
    let mut facts = Facts::default();
    match ability {
        AbilityDef::Triggered { effects, .. } => read_list(effects, true, reading, &mut facts),
        // The modes are chosen as it goes on the stack, so nothing is certain.
        AbilityDef::ModalTriggered { modes, .. } => {
            for mode in *modes {
                read_list(mode.effects, false, reading, &mut facts);
            }
        }
        _ => {}
    }
    facts
}

/// Reads a list. Only what stands on its own, or in a plain sequence, is
/// certain: a branch may not happen.
fn read_list(effects: &[Effect], certain: bool, reading: &Reading<'_>, facts: &mut Facts) {
    for effect in effects {
        read_one(effect, certain, reading, facts);
        let certain = certain && matches!(effect, Effect::Sequence(_));
        let (then, otherwise) = effect.branches();
        read_list(then, certain, reading, facts);
        read_list(otherwise, certain, reading, facts);
    }
}

/// What an amount counts as the trigger resolves.
const fn counted(amount: Amount) -> Stuff {
    match amount {
        Amount::CountersOnSource(..) => Stuff::OWN_COUNTERS,
        Amount::CountersOnYou(..) => Stuff::COUNTERS,
        Amount::CountOf { .. }
        | Amount::ControlledCount { .. }
        | Amount::DistinctColorsAmong(..)
        | Amount::BasicLandTypesAmong(..)
        | Amount::DevotionToManaColor => Stuff::BOARD,
        Amount::HandSize(..) => Stuff::HAND,
        Amount::LifeTotal(..) => Stuff::LIFE,
        Amount::SourcePower | Amount::TargetPower | Amount::EventPower => Stuff::POWER,
        _ => Stuff(0),
    }
}

/// Life gained by these players.
fn gains(
    amount: Amount,
    who: &[PlayerId],
    certain: bool,
    reading: &Reading<'_>,
    facts: &mut Facts,
) {
    facts.reads.add(counted(amount));
    for &seat in who {
        if seat == reading.me {
            facts.deeds.add(Deeds::HEALS_ME);
            facts.makes.add(Stuff::LIFE);
        }
        if let (true, Some(life)) = (
            certain,
            reading.fixed(amount).and_then(|n| i32::try_from(n).ok()),
        ) {
            facts.changes.push(Change {
                who: seat,
                what: Total::Life(life),
            });
        }
    }
}

/// Life taken from these players, damage or loss alike.
fn costs(
    amount: Amount,
    who: &[PlayerId],
    certain: bool,
    reading: &Reading<'_>,
    facts: &mut Facts,
) {
    facts.reads.add(counted(amount));
    for &seat in who {
        facts.deeds.add(if seat == reading.me {
            Deeds::HURTS_ME
        } else {
            Deeds::HURTS_OPPONENT
        });
        if let (true, Some(life)) = (
            certain,
            reading.fixed(amount).and_then(|n| i32::try_from(n).ok()),
        ) {
            facts.changes.push(Change {
                who: seat,
                what: Total::Life(-life),
            });
        }
    }
}

/// Cards drawn by these players.
fn draws(
    amount: Amount,
    who: &[PlayerId],
    certain: bool,
    reading: &Reading<'_>,
    facts: &mut Facts,
) {
    facts.reads.add(counted(amount));
    for &seat in who {
        if seat == reading.me {
            facts.deeds.add(Deeds::DRAWS);
            facts.makes.add(Stuff::HAND);
        }
        if let (true, Some(cards)) = (certain, reading.fixed(amount)) {
            facts.changes.push(Change {
                who: seat,
                what: Total::Draws(cards),
            });
        }
    }
}

/// Whether a printed boost shrinks rather than grows.
const fn shrinks(power: Amount, toughness: Amount) -> bool {
    matches!(power, Amount::NegXFixed(_) | Amount::NegX)
        || matches!(toughness, Amount::NegXFixed(_) | Amount::NegX)
}

#[allow(clippy::too_many_lines)] // one arm per group of verbs; the table is the point
fn read_one(effect: &Effect, certain: bool, reading: &Reading<'_>, facts: &mut Facts) {
    match *effect {
        // The player's totals.
        Effect::GainLife { amount } => gains(amount, &[reading.me], certain, reading, facts),
        Effect::GainLifeFor { amount, who } => {
            gains(amount, &reading.players(who), certain, reading, facts);
        }
        Effect::LoseLife { amount, target } => {
            costs(amount, &reading.players(target), certain, reading, facts);
        }
        Effect::DealDamage { amount, target } | Effect::GrantorDealsDamage { amount, target } => {
            facts.reads.add(counted(amount));
            match reading.struck(target) {
                Struck::Players(struck) => costs(amount, &struck, certain, reading, facts),
                Struck::Open => {}
                Struck::Object => facts.deeds.add(Deeds::REMOVES),
            }
        }
        Effect::Poison { who, amount } => {
            facts.reads.add(counted(amount));
            for seat in reading.players(who) {
                facts.deeds.add(if seat == reading.me {
                    Deeds::HURTS_ME
                } else {
                    Deeds::HURTS_OPPONENT
                });
                if let (true, Some(count)) = (certain, reading.fixed(amount)) {
                    facts.changes.push(Change {
                        who: seat,
                        what: Total::Poison(count),
                    });
                }
            }
        }
        Effect::WinGame if certain => facts.changes.push(Change {
            who: reading.me,
            what: Total::Wins,
        }),
        Effect::LoseGame if certain => facts.changes.push(Change {
            who: reading.me,
            what: Total::Loses,
        }),
        Effect::LoseHalfLife { .. }
        | Effect::SetLifeTotal { .. }
        | Effect::ExchangeLifeTotals { .. } => {
            facts.reads.add(Stuff::LIFE);
        }

        // Cards.
        Effect::DrawCards { amount } => draws(amount, &[reading.me], certain, reading, facts),
        Effect::DrawCardsFor { amount, who } => {
            draws(amount, &reading.players(who), certain, reading, facts);
        }
        Effect::DrawThenDiscardDrawn { .. } | Effect::DrawRevealDiscardUnless { .. } => {
            facts.deeds.add(Deeds::DRAWS);
            facts.makes.add(Stuff::HAND);
        }
        Effect::Scry { .. }
        | Effect::ScryFor { .. }
        | Effect::LookAtTopFind { .. }
        | Effect::LookAtTopExileFaceDown { .. }
        | Effect::LookAtTopPick { .. }
        | Effect::LookAtTopPickThenReorder { .. }
        | Effect::LookAtTopMayPut { .. }
        | Effect::LookAtTopMayTake { .. }
        | Effect::LookAtTopOf { .. }
        | Effect::LookAtTopKeepBottomPlay { .. }
        | Effect::ReorderTopLibrary { .. }
        | Effect::ReorderTopLibraryOf { .. }
        | Effect::NextDrawLooksAtTop { .. } => facts.deeds.add(Deeds::LOOKS),
        Effect::Surveil { .. } => {
            facts.deeds.add(Deeds::LOOKS);
            facts.makes.add(Stuff::GRAVEYARD);
        }
        Effect::SearchLibrary { .. }
        | Effect::SearchLibraryFor { .. }
        | Effect::SearchLibraryOf { .. }
        | Effect::SearchLibraryUpTo { .. }
        | Effect::SearchLibraryOrGraveyard { .. }
        | Effect::SearchSharedLandType
        | Effect::SearchWithManaDifference { .. }
        | Effect::RevealLibrarySearch { .. }
        | Effect::OptionalBasicLandSearchFor { .. } => {
            facts.deeds.add(Deeds::SEARCHES);
            facts.deeds.add(Deeds::SHUFFLES);
        }
        Effect::ShuffleLibrary { .. }
        | Effect::ShuffleIntoLibrary { .. }
        | Effect::ShuffleGraveyardIntoLibrary
        | Effect::ShuffleTargetsIntoOwnersLibraries => facts.deeds.add(Deeds::SHUFFLES),
        Effect::Mill { amount, target } => {
            facts.reads.add(counted(amount));
            if reading.players(target).contains(&reading.me) {
                facts.makes.add(Stuff::GRAVEYARD);
            }
        }

        // What the player is asked as it resolves.
        Effect::MayDo { .. } | Effect::MayDoFor { .. } | Effect::MayDoOnceEachTurn { .. } => {
            facts.deeds.add(Deeds::ASKS);
            facts.deeds.add(Deeds::MAY);
        }
        Effect::ChooseOne { .. }
        | Effect::ChooseNumber { .. }
        | Effect::ChooseCardName { .. }
        | Effect::ChooseColors { .. }
        | Effect::DiscardCount { .. }
        | Effect::DiscardMatching { .. }
        | Effect::DiscardUpToThenDraw { .. }
        | Effect::DiscardAnyThenDraw
        | Effect::Connive { .. }
        | Effect::Explore { .. } => facts.deeds.add(Deeds::ASKS),
        Effect::DiscardForPlayers { who, .. }
        | Effect::DiscardAmountForPlayers { who, .. }
        | Effect::SacrificeAmountForPlayers { who, .. }
        | Effect::SacrificeFilter { who, .. } => {
            if reading.players(who).contains(&reading.me) {
                facts.deeds.add(Deeds::ASKS);
            }
        }
        Effect::PlayerMayPayOr { .. }
        | Effect::PlayerMayPayManaOr { .. }
        | Effect::PlayerMayPayManaThen { .. }
        | Effect::PlayerMayPayManaEachThenElse { .. }
        | Effect::PlayerMayPayManaAndLifeEachThenElse { .. }
        | Effect::PlayerMayPayThen { .. }
        | Effect::PlayerMayPayThenElse { .. }
        | Effect::PlayerMayPayCostOr { .. }
        | Effect::PlayerMayPayCostThen { .. }
        | Effect::PayManaOrLose { .. } => {
            facts.deeds.add(Deeds::ASKS);
            facts.reads.add(Stuff::MANA);
        }

        // What is taken away.
        Effect::Destroy { .. }
        | Effect::DestroyAll { .. }
        | Effect::DestroyAllExceptChosen { .. }
        | Effect::DestroyAllWithManaValue { .. }
        | Effect::DestroyRandom { .. }
        | Effect::DestroyTargetSlots { .. }
        | Effect::DestroyWeakest { .. }
        | Effect::Exile { .. }
        | Effect::ExileAll { .. }
        | Effect::ReturnToHand { .. }
        | Effect::ReturnAllToHand { .. }
        | Effect::PutTargetOnTopOfLibrary
        | Effect::PutTargetOnBottomOfLibrary
        | Effect::RemoveTargetFromCombat { .. }
        | Effect::DealDamageEach { .. }
        | Effect::DealDamageDivided { .. } => facts.deeds.add(Deeds::REMOVES),
        Effect::SacrificeSelf
        | Effect::ExileSource
        | Effect::ExileSelfReturnAsFace { .. }
        | Effect::ExileSourceWithCounters { .. }
        | Effect::MoveSourceToOwnerGraveyard
        | Effect::PutSourceOnTopOfLibrary => facts.deeds.add(Deeds::LEAVES),

        // What is made bigger or brought in.
        Effect::PumpTarget {
            power, toughness, ..
        } => {
            facts.reads.add(counted(power));
            if shrinks(power, toughness) {
                facts.deeds.add(Deeds::REMOVES);
            } else {
                facts.deeds.add(Deeds::PUMPS);
                facts.makes.add(Stuff::POWER);
            }
        }
        Effect::PumpFilter { .. } | Effect::ModifyFilter { .. } => {
            facts.deeds.add(Deeds::PUMPS);
            facts.reads.add(Stuff::BOARD);
            facts.makes.add(Stuff::POWER);
        }
        Effect::PumpEventObject { .. }
        | Effect::PumpTargetSlot { .. }
        | Effect::ModifySource { .. }
        | Effect::ModifyTargets { .. }
        | Effect::ModifyAttached { .. }
        | Effect::ModifyEvent { .. } => {
            facts.deeds.add(Deeds::PUMPS);
            facts.makes.add(Stuff::POWER);
        }
        Effect::AddCounter { amount, .. } => {
            facts.reads.add(counted(amount));
            facts.deeds.add(Deeds::PUMPS);
            facts.makes.add(Stuff::COUNTERS);
            facts.makes.add(Stuff::POWER);
        }
        Effect::AddCounterSource { amount, .. } => {
            facts.reads.add(counted(amount));
            facts.deeds.add(Deeds::PUMPS);
            facts.makes.add(Stuff::OWN_COUNTERS);
            facts.makes.add(Stuff::POWER);
        }
        Effect::AddCounterFilter { .. }
        | Effect::AddCounterCreatedTokens { .. }
        | Effect::AddCountersUpTo { .. } => {
            facts.deeds.add(Deeds::PUMPS);
            facts.makes.add(Stuff::COUNTERS);
            facts.makes.add(Stuff::POWER);
        }
        Effect::Proliferate => {
            facts.reads.add(Stuff::COUNTERS);
            facts.makes.add(Stuff::COUNTERS);
        }
        Effect::CreateTokenN { amount, .. } => {
            facts.reads.add(counted(amount));
            facts.makes.add(Stuff::BOARD);
        }
        Effect::CreateToken { .. }
        | Effect::CreateTokenAttacking { .. }
        | Effect::CreateTokenModified { .. }
        | Effect::CreateTokensFor { .. }
        | Effect::CreateTokenCopyOf { .. }
        | Effect::CreateTokenCopiesOfGraveyard { .. }
        | Effect::CreateTokenCopyOfSource { .. }
        | Effect::CreateTokenCopyOfTarget { .. }
        | Effect::CreateTokenThenCounters { .. }
        | Effect::Populate
        | Effect::Amass { .. }
        | Effect::Fabricate { .. }
        | Effect::PutFromHandOntoBattlefield { .. }
        | Effect::GraveyardToBattlefield { .. }
        | Effect::GraveyardToBattlefieldModified { .. }
        | Effect::YourGraveyardToBattlefield { .. } => facts.makes.add(Stuff::BOARD),
        Effect::AddMana { .. }
        | Effect::AddManaFor { .. }
        | Effect::AddManaAmountFor { .. }
        | Effect::AddManaLikeEvent { .. }
        | Effect::ManaForColorsAmong { .. } => facts.makes.add(Stuff::MANA),

        // What reaches back into a graveyard as it resolves.
        Effect::ChooseFromYourGraveyard { .. }
        | Effect::ReturnChosenToHand { .. }
        | Effect::RandomGraveyardToHand { .. }
        | Effect::GraveyardToHand { .. }
        | Effect::GraveyardToTop { .. }
        | Effect::GraveyardAllToHand { .. } => facts.reads.add(Stuff::GRAVEYARD),

        // What fights reads the power it fights with.
        Effect::Fight { .. }
        | Effect::DamageEqualToPower { .. }
        | Effect::EventObjectDealsDamageEqualToPower { .. } => {
            facts.deeds.add(Deeds::REMOVES);
            facts.reads.add(Stuff::POWER);
        }
        _ => {}
    }
}
