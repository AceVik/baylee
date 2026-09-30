//! Damage dealt to a player, as the game remembers it: the turn's tally
//! ("the damage dealt to you this turn") and the trigger ("whenever you're
//! dealt damage"), both fed at the one door damage to a player comes
//! through; and "you may [pay]. If you do, …" (CR 118.12), asked only while
//! the payment is possible (CR 608.2d).
//!
//! Played with permanents built for it, each with the sentences under test,
//! beside vanilla attackers.

use super::synthetic::{SyntheticLookup, creature_face, keep_mulligans, permanents, preset_both};
use super::*;
use crate::event::{Cause, GameEvent};
use crate::zone::{ZoneLocation, ZonePosition};
use baylee_cards_dsl::{
    AbilityDef, Amount, CardDef, CommanderRule, Coverage, Effect, FaceDef, KeywordSet, PlayerRel,
    StepKind, Trigger, counters, triggered,
};
use baylee_core::color::ColorSet;
use baylee_core::ids::{CardIndex, Defender};

// ---------------------------------------------------------------- fixtures

/// A 0/4: "Whenever you're dealt damage, put that many vitality counters
/// on this. At the beginning of your upkeep, you may remove a vitality
/// counter from this. If you do, you gain 1 life."
const VITAL: u32 = 1170;
/// A 0/1: "At the beginning of your upkeep, you may sacrifice this. If you
/// do, you gain 3 life."
const IDOL: u32 = 1171;
/// A vanilla 2/2.
const BEAR: u32 = 1172;
/// A vanilla 3/3.
const OGRE: u32 = 1173;
/// A 1/1 with first strike.
const STRIKER: u32 = 1174;

static VITALITY: &[AbilityDef] = &[
    triggered!(
        Trigger::PlayerDealtDamage(PlayerRel::You),
        &[Effect::AddCounter {
            kind: counters::VITALITY,
            amount: Amount::EventAmount,
        }],
    ),
    triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::You,
        },
        &[Effect::MayDo {
            effects: &[
                Effect::RemoveCounterSelf {
                    kind: counters::VITALITY,
                    n: 1,
                },
                Effect::GainLife {
                    amount: Amount::Fixed(1),
                },
            ],
        }],
    ),
];
static OFFERING: &[AbilityDef] = &[triggered!(
    Trigger::StepBegin {
        step: StepKind::Upkeep,
        whose: PlayerRel::You,
    },
    &[Effect::MayDo {
        effects: &[
            Effect::SacrificeSelf,
            Effect::GainLife {
                amount: Amount::Fixed(3),
            },
        ],
    }],
)];

fn body(
    index: u32,
    name: &'static str,
    (power, toughness): (i16, i16),
    keywords: KeywordSet,
    abilities: &'static [AbilityDef],
) -> &'static CardDef {
    Box::leak(Box::new(CardDef {
        index: CardIndex::new(index),
        oracle_id: "test",
        scryfall_id: "test",
        faces: Box::leak(Box::new([FaceDef {
            abilities,
            keywords,
            ..creature_face(name, power, toughness, &[])
        }])),
        color_identity: ColorSet::EMPTY,
        keywords,
        commander: CommanderRule::NotEligible,
        partner: baylee_cards_dsl::PartnerKind::None,
        coverage: Coverage::Implemented,
        abilities,
    }))
}

fn lookup() -> SyntheticLookup {
    let none = KeywordSet::EMPTY;
    SyntheticLookup::new(vec![
        body(VITAL, "Vital", (0, 4), none, VITALITY),
        body(IDOL, "Idol", (0, 1), none, OFFERING),
        body(BEAR, "Bear", (2, 2), none, &[]),
        body(OGRE, "Ogre", (3, 3), none, &[]),
        body(STRIKER, "Striker", (1, 1), KeywordSet::FIRST_STRIKE, &[]),
    ])
}

const ME: PlayerId = PlayerId::new(0);
const THEM: PlayerId = PlayerId::new(1);

fn start(mine: &[u32], theirs: &[u32]) -> Engine<SyntheticLookup> {
    let mut engine = Engine::new(&preset_both(118, mine, theirs), lookup()).unwrap();
    keep_mulligans(&mut engine);
    engine
}

/// Walks the game until `stop` holds, attacking `THEM` with every one of
/// `attackers` in turn 1 and with nothing after; nobody blocks, and every
/// other question is refused by panicking.
fn walk(
    engine: &mut Engine<SyntheticLookup>,
    attackers: &[ObjectId],
    stop: impl Fn(&Engine<SyntheticLookup>) -> bool,
) {
    for _ in 0..400 {
        if stop(engine) {
            return;
        }
        match engine.pending().clone() {
            Pending::ChooseAttackers { player, .. }
                if player == ME && engine.state().turn.number == 1 =>
            {
                let attackers = attackers
                    .iter()
                    .map(|a| (*a, Defender::Player(THEM)))
                    .collect();
                engine
                    .apply(ME, PlayerAction::DeclareAttackers { attackers })
                    .unwrap();
            }
            other => assert!(
                super::synthetic::walk_past(engine, &other),
                "an unexpected question: {other:?}"
            ),
        }
    }
    panic!("the game never got there");
}

fn vitality(engine: &Engine<SyntheticLookup>, id: ObjectId) -> u16 {
    engine
        .state()
        .object(id)
        .map_or(0, |o| o.counters.get(counters::VITALITY))
}

/// How many times the counters on `id` changed in the whole game.
fn counter_changes(engine: &Engine<SyntheticLookup>, id: ObjectId) -> usize {
    engine
        .state()
        .journal
        .entries()
        .iter()
        .filter(|e| matches!(e.event, GameEvent::CounterChanged { object, .. } if object == id))
        .count()
}

fn their_upkeep(engine: &Engine<SyntheticLookup>) -> bool {
    engine.state().turn.active == THEM && engine.state().turn.step == crate::turn::Step::Upkeep
}

fn my_end_step(engine: &Engine<SyntheticLookup>) -> bool {
    engine.state().turn.active == ME && engine.state().turn.step == crate::turn::Step::End
}

fn life(engine: &Engine<SyntheticLookup>, seat: PlayerId) -> i32 {
    engine.state().players[seat.get() as usize].life
}

// --------------------------------------------------------------- damage

/// Two unblocked attackers are one damage event (CR 510.2, 603.2c): the
/// trigger goes on the stack once, and "that many" is both creatures'
/// damage. The turn's tally is the same 5, and the next turn starts it from
/// nothing. On the old engine nothing triggered and nothing was counted.
#[test]
fn two_attackers_are_one_trigger_for_all_their_damage() {
    let mut engine = start(&[BEAR, OGRE], &[VITAL]);
    let (bear, ogre) = (permanents(&engine, BEAR)[0], permanents(&engine, OGRE)[0]);
    let vital = permanents(&engine, VITAL)[0];
    let start_life = life(&engine, THEM);

    walk(&mut engine, &[bear, ogre], my_end_step);

    assert_eq!(life(&engine, THEM), start_life - 5);
    assert_eq!(vitality(&engine, vital), 5, "that many: all of it");
    assert_eq!(counter_changes(&engine, vital), 1, "from one trigger");
    assert_eq!(
        engine.state().per_turn.damage_dealt_to,
        vec![0, 5],
        "the turn's tally, per seat"
    );

    walk(&mut engine, &[], their_upkeep);
    assert_eq!(
        engine.state().per_turn.damage_dealt_to,
        vec![0, 0],
        "a new turn counts from nothing"
    );
}

/// First strike makes a second combat damage step (CR 510.4), and its
/// damage is another event: two triggers, each for its own step's damage.
#[test]
fn a_first_strike_step_is_an_event_of_its_own() {
    let mut engine = start(&[STRIKER, BEAR], &[VITAL]);
    let (striker, bear) = (
        permanents(&engine, STRIKER)[0],
        permanents(&engine, BEAR)[0],
    );
    let vital = permanents(&engine, VITAL)[0];

    walk(&mut engine, &[striker, bear], my_end_step);

    assert_eq!(vitality(&engine, vital), 3);
    assert_eq!(counter_changes(&engine, vital), 2, "one trigger per step");
    assert_eq!(engine.state().per_turn.damage_dealt_to[1], 3);
}

// ------------------------------------------------------- if you do

/// "You may remove a vitality counter from this. If you do, you gain 1
/// life": asked while a counter is there, and a yes pays and gains.
#[test]
fn a_counter_there_to_remove_is_asked_about_and_paid() {
    let mut engine = start(&[BEAR], &[VITAL]);
    let vital = permanents(&engine, VITAL)[0];
    engine
        .dev_state_mut(THEM)
        .unwrap()
        .object_mut(vital)
        .unwrap()
        .counters
        .set(counters::VITALITY, 2);

    walk(&mut engine, &[], |e| {
        their_upkeep(e) && matches!(e.pending(), Pending::YesNo { .. })
    });
    let before = life(&engine, THEM);
    engine.apply(THEM, PlayerAction::YesNo(true)).unwrap();

    assert_eq!(vitality(&engine, vital), 1, "one came off");
    assert_eq!(life(&engine, THEM), before + 1);
}

/// With no counter to remove the yes is impossible, so nobody is asked
/// (CR 608.2d) and the ability does nothing; the same when the permanent
/// has left the battlefield with its trigger on the stack, even with
/// counters on the card it became: the card in the graveyard is a new
/// object (CR 400.7), and nothing is removed from this permanent by taking
/// them off that one. (A trigger that put counters on "this" after it left
/// leaves them on the card; the stack object carries no version to tell.)
#[test]
fn nothing_to_remove_asks_nothing() {
    for gone in [false, true] {
        let mut engine = start(&[BEAR], &[VITAL]);
        let vital = permanents(&engine, VITAL)[0];
        walk(&mut engine, &[], |e| {
            their_upkeep(e) && !e.state().zones.list(ZoneLocation::Stack).is_empty()
        });
        if gone {
            let state = engine.dev_state_mut(THEM).unwrap();
            state
                .move_object(
                    vital,
                    ZoneLocation::Graveyard(THEM),
                    ZonePosition::Top,
                    Cause::DevCommand,
                )
                .unwrap();
            state
                .object_mut(vital)
                .unwrap()
                .counters
                .set(counters::VITALITY, 2);
            engine.refresh_offer();
        }
        let before = life(&engine, THEM);
        walk(&mut engine, &[], |e| {
            assert!(
                !matches!(e.pending(), Pending::YesNo { .. }),
                "asked with the payment impossible (gone: {gone})"
            );
            e.state().zones.list(ZoneLocation::Stack).is_empty()
        });
        assert_eq!(life(&engine, THEM), before, "and nothing gained");
    }
}

/// "You may sacrifice this. If you do, you gain 3 life" with the permanent
/// already gone: the sacrifice cannot be paid, so the question is not put
/// and the life is not gained (CR 118.12, 608.2d). The guard used to match
/// a `MayDo` holding the sacrifice alone, so this one, with what the
/// sacrifice buys after it, was asked and a yes gained the 3 for nothing.
#[test]
fn a_sacrifice_that_cannot_be_paid_buys_nothing() {
    let mut engine = start(&[BEAR], &[IDOL]);
    let idol = permanents(&engine, IDOL)[0];
    walk(&mut engine, &[], |e| {
        their_upkeep(e) && !e.state().zones.list(ZoneLocation::Stack).is_empty()
    });
    engine
        .dev_state_mut(THEM)
        .unwrap()
        .move_object(
            idol,
            ZoneLocation::Hand(THEM),
            ZonePosition::Top,
            Cause::DevCommand,
        )
        .unwrap();
    engine.refresh_offer();
    let before = life(&engine, THEM);

    walk(&mut engine, &[], |e| {
        assert!(
            !matches!(e.pending(), Pending::YesNo { .. }),
            "a sacrifice nobody can make is not offered"
        );
        e.state().zones.list(ZoneLocation::Stack).is_empty()
    });
    assert_eq!(life(&engine, THEM), before);
}

/// The counter-check: on the battlefield the sacrifice is offered, and a
/// yes pays it and gains the life.
#[test]
fn a_sacrifice_that_can_be_paid_is_offered() {
    let mut engine = start(&[BEAR], &[IDOL]);
    let idol = permanents(&engine, IDOL)[0];
    walk(&mut engine, &[], |e| {
        their_upkeep(e) && matches!(e.pending(), Pending::YesNo { .. })
    });
    let before = life(&engine, THEM);
    engine.apply(THEM, PlayerAction::YesNo(true)).unwrap();

    assert_eq!(life(&engine, THEM), before + 3);
    assert_ne!(
        engine.state().object(idol).map(|o| o.zone),
        Some(crate::zone::Zone::Battlefield),
        "sacrificed"
    );
}
