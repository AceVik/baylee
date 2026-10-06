//! One rule test per DSL mechanic the pool prints, played against a card
//! nobody printed.
//!
//! A mechanic that only card tests reach is a mechanic whose meaning is
//! whatever the one card that prints it happened to need. The files below
//! each take a sentence of the DSL — a trigger, a branch, an effect, a cost
//! part, a mana rider — and play it on a board built for it, with the
//! negative beside the positive: the moment the sentence must *not* fire is
//! what tells a working matcher from one that answers yes to everything.
//!
//! The bench is [`super::synthetic`]'s bargain, widened to both seats and to
//! a hand: every card here is leaked at a made-up index, the pool answers
//! for the Forests behind them, and nothing is asked of the engine except
//! through `pending()` and `apply()` once a player has something to decide.
//! Where a test sets a board up directly it says so, and it is the board and
//! never the rule that is set.

mod blocks;
mod branches;
mod control;
mod costs;
mod effects;
mod mana;
mod tokens;
mod transforms;
mod triggers;
mod zones;

use super::synthetic::{SyntheticLookup, forest, land_face, walk_past};
use super::*;
use baylee_cards_dsl::{
    AbilityDef, ActivationLimit, ActivationTiming, ActivationZone, CardDef, CommanderRule, Cost,
    Coverage, Effect, FaceDef, KeywordSet, PartnerKind, TargetReq,
};
use baylee_core::ids::{CardIndex, PrintRef};
use baylee_core::mana::ManaCost;
use baylee_core::preset::{
    AIProfile, DeckEntry, Finish, FormatId, GamePreset, PrintInfo, SeatController, SeatSpec,
};
use baylee_core::types::TypeSet;

/// The engine these tests drive.
pub(super) type Bench = Engine<SyntheticLookup>;

/// The seat that starts the game (`TurnInfo::new` is seat 0's turn 1).
pub(super) fn me() -> PlayerId {
    PlayerId::new(0)
}

/// The other seat.
pub(super) fn them() -> PlayerId {
    PlayerId::new(1)
}

// ----------------------------------------------------------------- cards

/// A face with a name, a cost and a type line, and every other
/// characteristic at the value a card that prints nothing for it has.
///
/// [`land_face`] is the one place those defaults are written; this only
/// says what makes a spell a spell.
pub(super) fn face(name: &'static str, cost: &'static str, types: TypeSet) -> FaceDef {
    FaceDef {
        mana_cost: ManaCost::parse(cost),
        types,
        castable_from_hand: true,
        ..land_face(name)
    }
}

/// A creature face: [`face`] with a body.
pub(super) fn creature_face(
    name: &'static str,
    cost: &'static str,
    power: i16,
    toughness: i16,
) -> FaceDef {
    FaceDef {
        power: Some(power),
        toughness: Some(toughness),
        ..face(name, cost, TypeSet::CREATURE)
    }
}

/// A one-faced card at a made-up index, leaked so the engine can hold it for
/// `'static` the way it holds a compiled card.
pub(super) fn card(
    index: u32,
    face: FaceDef,
    keywords: KeywordSet,
    abilities: &'static [AbilityDef],
) -> &'static CardDef {
    assert_ne!(
        index,
        forest(),
        "a made-up index must not shadow the pool's Forest, which fills every \
         library here"
    );
    Box::leak(Box::new(CardDef {
        index: CardIndex::new(index),
        oracle_id: "test",
        scryfall_id: "test",
        faces: Box::leak(Box::new([face])),
        color_identity: baylee_core::color::ColorSet::EMPTY,
        keywords,
        commander: CommanderRule::NotEligible,
        partner: PartnerKind::None,
        coverage: Coverage::Implemented,
        abilities,
    }))
}

/// "{0}: `effects`" — an activated ability that costs nothing, at instant
/// speed, from the battlefield.
///
/// The probe most of these files hang their sentence on. Free, so no mana
/// stands between the test and the effect; an activated ability rather than
/// a spell, so there is no hand, no cast and no timing to get past.
pub(super) const fn free(effects: &'static [Effect], targets: Option<TargetReq>) -> AbilityDef {
    paid(Cost::FREE, effects, targets)
}

/// The same probe, behind a cost.
pub(super) const fn paid(
    cost: Cost,
    effects: &'static [Effect],
    targets: Option<TargetReq>,
) -> AbilityDef {
    AbilityDef::Activated {
        cost,
        effects,
        targets,
        second_targets: None,
        timing: ActivationTiming::InstantSpeed,
        mana_ability: false,
        zone: ActivationZone::Battlefield,
        limit: ActivationLimit::Unlimited,
        cost_reduction: None,
    }
}

// ----------------------------------------------------------------- boards

/// What one seat starts with.
#[derive(Default, Clone)]
pub(super) struct Seat {
    pub battlefield: Vec<u32>,
    pub hand: Vec<u32>,
    pub life: Option<i32>,
}

impl Seat {
    /// A seat with these permanents.
    pub(super) fn with(battlefield: &[u32]) -> Self {
        Self {
            battlefield: battlefield.to_vec(),
            ..Self::default()
        }
    }

    /// And these cards in hand.
    pub(super) fn holding(mut self, hand: &[u32]) -> Self {
        self.hand = hand.to_vec();
        self
    }

    /// And this life total.
    pub(super) const fn at(mut self, life: i32) -> Self {
        self.life = Some(life);
        self
    }
}

fn entry(card: u32) -> DeckEntry {
    DeckEntry {
        card: CardIndex::new(card),
        print: PrintRef::new(0),
    }
}

/// A two-seat game over these cards: the seats as given, Forests in both
/// libraries, and both opening hands kept.
pub(super) fn bench(seed: u64, cards: Vec<&'static CardDef>, seats: [Seat; 2]) -> Bench {
    let deck: Vec<DeckEntry> = (0..60).map(|_| entry(forest())).collect();
    let seat = |s: Seat| SeatSpec {
        controller: SeatController::Ai(AIProfile::default()),
        capabilities: baylee_core::preset::SeatCapabilities {
            dev_commands: true,
            see_hidden: false,
        },
        deck: deck.clone(),
        sideboard: vec![],
        commanders: vec![],
        starting_life: s.life,
        starting_hand: Some(s.hand.into_iter().map(entry).collect()),
        starting_battlefield: s.battlefield.into_iter().map(entry).collect(),
        emblems: vec![],
        team: None,
    };
    let preset = GamePreset {
        format: FormatId::Freeform,
        seed,
        house_rules: HouseRules::default(),
        modifiers: vec![],
        prints: vec![PrintInfo {
            scryfall_id: uuid::Uuid::nil(),
            lang: "EN".into(),
            finish: Finish::Normal,
        }],
        seats: seats.into_iter().map(seat).collect(),
    };
    let mut engine = Engine::new(&preset, SyntheticLookup::new(cards)).expect("a two-seat game");
    super::synthetic::keep_mulligans(&mut engine);
    engine
}

// ---------------------------------------------------------------- reading

/// Every object of one card index in one zone.
pub(super) fn objects(engine: &Bench, zone: ZoneLocation, index: u32) -> Vec<ObjectId> {
    engine
        .state()
        .zones
        .list(zone)
        .iter()
        .copied()
        .filter(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index.get() == index))
        })
        .collect()
}

/// The one object of this card index in this zone.
#[track_caller]
pub(super) fn the(engine: &Bench, zone: ZoneLocation, index: u32) -> ObjectId {
    let found = objects(engine, zone, index);
    assert_eq!(found.len(), 1, "exactly one {index} in {zone:?}: {found:?}");
    found[0]
}

/// A seat's life total.
pub(super) fn life(engine: &Bench, seat: PlayerId) -> i32 {
    engine.state().players[seat.get() as usize].life
}

/// How many times the triggered ability printed first on `source` has
/// triggered this game — each trigger in these files is its card's
/// ability 0.
///
/// Read off the journal rather than off what the ability did, because a
/// trigger that never went on the stack and one that went on and did
/// nothing end with the same board. The index is asked as well as the
/// source because an *activation* is journalled under the same event: the
/// Raider's own "{0}: deal 1 damage" goes on the stack as its ability 1.
pub(super) fn triggered(engine: &Bench, source: ObjectId) -> usize {
    engine
        .journal()
        .entries()
        .iter()
        .filter(|e| {
            matches!(
                e.event,
                crate::event::GameEvent::AbilityTriggered { source: s, ability_index: 0, .. }
                    if s == source
            )
        })
        .count()
}

/// The offer the seat holding priority has right now.
#[track_caller]
pub(super) fn offer(engine: &Bench) -> (PlayerId, LegalActions) {
    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    (player, *legal)
}

// ---------------------------------------------------------------- driving

/// Answers the turn's own questions (priority, empty attacks, no blocks)
/// until `done` holds, and panics on anything else: a question the board
/// under test was not built to ask is a finding, not something to answer.
#[track_caller]
pub(super) fn walk_until(engine: &mut Bench, done: impl Fn(&Bench) -> bool) {
    for _ in 0..600 {
        if done(engine) {
            return;
        }
        let pending = engine.pending().clone();
        assert!(
            walk_past(engine, &pending),
            "unexpected question: {pending:?}"
        );
    }
    panic!("never got there");
}

/// Whether `seat` holds priority in `step` of its own turn with nothing on
/// the stack.
pub(super) fn holds_priority_in(engine: &Bench, seat: PlayerId, step: crate::turn::Step) -> bool {
    engine.state().turn.active == seat
        && engine.state().turn.step == step
        && engine.state().zones.stack_is_empty()
        && matches!(engine.pending(), Pending::Priority { player, .. } if *player == seat)
}

/// Walks to `seat`'s next main phase, holding priority on an empty stack.
#[track_caller]
pub(super) fn to_main(engine: &mut Bench, seat: PlayerId) {
    walk_until(engine, |e| {
        holds_priority_in(e, seat, crate::turn::Step::Main)
            && e.state().turn.phase == Phase::FirstMain
    });
}

/// Passes priority until the stack is empty and `seat` holds priority again.
#[track_caller]
pub(super) fn settle(engine: &mut Bench, seat: PlayerId) {
    walk_until(engine, |e| {
        e.state().zones.stack_is_empty()
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == seat)
    });
}

/// Activates ability `index` of `source` for `seat`, through the offer.
#[track_caller]
pub(super) fn activate(engine: &mut Bench, seat: PlayerId, source: ObjectId, index: u32) {
    let (player, legal) = offer(engine);
    assert_eq!(player, seat, "the seat activating holds priority");
    assert!(
        legal.abilities.contains(&(source, index)),
        "the engine offers ability {index} of {source:?}: {:?}",
        legal.abilities
    );
    engine
        .apply(
            seat,
            PlayerAction::ActivateAbility {
                source,
                ability_index: index,
            },
        )
        .expect("the engine takes the ability it offered");
}

/// Taps every land `seat` controls for its intrinsic mana.
pub(super) fn float_all(engine: &mut Bench, seat: PlayerId) {
    let (player, legal) = offer(engine);
    assert_eq!(player, seat, "the seat tapping holds priority");
    for source in legal.mana_abilities {
        engine
            .apply(seat, PlayerAction::ActivateManaAbility { source })
            .expect("a land taps for its own mana");
    }
}
