//! Three effects that leave something behind for later: a flashback grant
//! (`GrantFlashback`), the prepared marker (`BecomePrepared`) and a debt
//! due at the next upkeep (`PayCostOrLoseLater`).
//!
//! Each is played from the resolution that creates it to the moment it is
//! read, because the effect is only half of the sentence — "gains flashback
//! until end of turn" is a card that may be cast and then may not, and a
//! debt is a question asked at one upkeep and not at another.

use super::*;
use baylee_cards_dsl::{Filter, PlayerRel, TargetSpec};

const SNAPPER: u32 = 7140;
const SPARK: u32 = 7141;
const STUDY: u32 = 7142;
const LESSON: u32 = 7143;
const PACT: u32 = 7144;

static GAIN_ONE: &[Effect] = &[Effect::gain_life(1)];
static SPELL_GAIN_ONE: &[AbilityDef] = &[AbilityDef::Spell {
    effects: GAIN_ONE,
    targets: None,
    second_targets: None,
    condition: None,
}];

static INSTANT_OR_SORCERY: Filter = Filter::INSTANT_OR_SORCERY;

/// Snapcaster Mage's sentence on a free ability: "target instant or sorcery
/// card in your graveyard gains flashback until end of turn. The flashback
/// cost is equal to its mana cost."
static SNAPPER_ABILITIES: &[AbilityDef] = &[free(
    &[Effect::GrantFlashback],
    Some(TargetReq::one(TargetSpec::CardInGraveyard(
        &INSTANT_OR_SORCERY,
        PlayerRel::You,
    ))),
)];

/// Emeritus of Woe's two halves on one land: the spell it may cast while
/// prepared, and "{0}: this becomes prepared".
static STUDY_ABILITIES: &[AbilityDef] = &[
    AbilityDef::Prepared {
        card: CardIndex::new(LESSON),
    },
    free(&[Effect::BecomePrepared], None),
];

/// Pact of Negation's second sentence on a free ability: "At the beginning
/// of your next upkeep, pay {G}. If you don't, you lose the game."
static PACT_ABILITIES: &[AbilityDef] = &[free(
    &[Effect::PayCostOrLoseLater {
        cost: ManaCost::parse("{G}"),
    }],
    None,
)];

fn cards() -> Vec<&'static CardDef> {
    let land = super::super::synthetic::land;
    vec![
        land(SNAPPER, "Snapper", SNAPPER_ABILITIES),
        card(
            SPARK,
            face("Spark", "{0}", TypeSet::INSTANT),
            KeywordSet::EMPTY,
            SPELL_GAIN_ONE,
        ),
        land(STUDY, "Study", STUDY_ABILITIES),
        card(
            LESSON,
            face("Lesson", "{0}", TypeSet::INSTANT),
            KeywordSet::EMPTY,
            SPELL_GAIN_ONE,
        ),
        land(PACT, "Pact", PACT_ABILITIES),
    ]
}

// ------------------------------------------------------------- flashback

/// Seat 0 at its main phase with two Sparks cast and resolved, so both lie
/// in its graveyard, and the Snapper beside them.
fn two_sparks_spent(seed: u64) -> (Bench, ObjectId, [ObjectId; 2]) {
    let mut engine = bench(
        seed,
        cards(),
        [
            Seat::with(&[SNAPPER]).holding(&[SPARK, SPARK]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    let sparks: Vec<ObjectId> = objects(&engine, ZoneLocation::Hand(me()), SPARK);
    for &spark in &sparks {
        engine
            .apply(me(), PlayerAction::CastSpell { card: spark })
            .unwrap();
        settle(&mut engine, me());
    }
    assert_eq!(
        objects(&engine, ZoneLocation::Graveyard(me()), SPARK),
        sparks,
        "both Sparks resolved into the graveyard"
    );
    let snapper = the(&engine, ZoneLocation::Battlefield, SNAPPER);
    (engine, snapper, [sparks[0], sparks[1]])
}

/// Activates the Snapper at `target` and lets it resolve.
fn grant(engine: &mut Bench, snapper: ObjectId, target: ObjectId) {
    activate(engine, me(), snapper, 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("the grant targets: {:?}", engine.pending())
    };
    assert!(
        options.contains(&target),
        "a card in my graveyard: {options:?}"
    );
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![target],
                players: vec![],
            },
        )
        .unwrap();
    settle(engine, me());
}

/// The grant goes to the card it targeted and no other (CR 702.34a: "you
/// may cast this card from your graveyard"), and a spell cast that way is
/// exiled as it leaves the stack rather than returned to the graveyard.
#[test]
fn a_granted_card_is_cast_from_the_graveyard_and_exiled_after() {
    let (mut engine, snapper, [first, second]) = two_sparks_spent(7140);
    let (_, legal) = offer(&engine);
    assert!(
        !legal.castable.contains(&first) && !legal.castable.contains(&second),
        "nothing is castable from a graveyard before the grant: {:?}",
        legal.castable
    );

    grant(&mut engine, snapper, first);
    assert!(crate::casting::flashback_granted(engine.state(), first));
    assert!(
        !crate::casting::flashback_granted(engine.state(), second),
        "the grant names one card"
    );
    let (_, legal) = offer(&engine);
    assert!(
        legal.castable.contains(&first),
        "the targeted card is offered: {:?}",
        legal.castable
    );
    assert!(
        !legal.castable.contains(&second),
        "its twin beside it is not: {:?}",
        legal.castable
    );

    let before = life(&engine, me());
    engine
        .apply(me(), PlayerAction::CastSpell { card: first })
        .expect("the offer's card is cast");
    settle(&mut engine, me());
    assert_eq!(life(&engine, me()), before + 1, "and it resolved");
    assert_eq!(
        objects(&engine, ZoneLocation::Exile(me()), SPARK),
        vec![first],
        "a flashed-back spell is exiled instead of going anywhere else"
    );
    assert_eq!(
        objects(&engine, ZoneLocation::Graveyard(me()), SPARK),
        vec![second]
    );
}

/// "Until end of turn": the next turn the card is a card in a graveyard
/// again.
#[test]
fn a_flashback_grant_ends_with_the_turn() {
    let (mut engine, snapper, [first, _]) = two_sparks_spent(7141);
    grant(&mut engine, snapper, first);
    assert!(crate::casting::flashback_granted(engine.state(), first));

    walk_until(&mut engine, |e| {
        holds_priority_in(e, them(), crate::turn::Step::Upkeep)
    });
    engine.apply(them(), PlayerAction::PassPriority).unwrap();
    assert!(
        !crate::casting::flashback_granted(engine.state(), first),
        "the grant ended in the cleanup step"
    );
    let (player, legal) = offer(&engine);
    assert_eq!(player, me());
    assert!(
        !legal.castable.contains(&first),
        "and an instant in a graveyard is not castable on the next turn: {:?}",
        legal.castable
    );
}

// -------------------------------------------------------------- prepared

fn prepared_markers(engine: &Bench, id: ObjectId) -> usize {
    engine.state().object(id).map_or(0, |o| {
        o.riders
            .iter()
            .filter(|r| **r == crate::object::Rider::Prepared)
            .count()
    })
}

fn offers_prepared_cast(engine: &Bench, id: ObjectId) -> bool {
    offer(engine)
        .1
        .abilities
        .contains(&(id, crate::choice::PREPARED_CAST))
}

/// The effect puts the marker on its source, once however often it runs,
/// and the marker is what the prepared cast is offered on. Casting it takes
/// the marker off, and the effect puts it back.
#[test]
fn becoming_prepared_marks_the_source_once_and_opens_its_spell() {
    let mut engine = bench(7142, cards(), [Seat::with(&[STUDY]), Seat::default()]);
    to_main(&mut engine, me());
    let study = the(&engine, ZoneLocation::Battlefield, STUDY);
    assert_eq!(prepared_markers(&engine, study), 0);
    assert!(!offers_prepared_cast(&engine, study), "not prepared yet");

    activate(&mut engine, me(), study, 1);
    settle(&mut engine, me());
    assert_eq!(prepared_markers(&engine, study), 1, "prepared");
    assert!(
        offers_prepared_cast(&engine, study),
        "and its spell is offered"
    );

    activate(&mut engine, me(), study, 1);
    settle(&mut engine, me());
    assert_eq!(
        prepared_markers(&engine, study),
        1,
        "prepared again is still prepared once"
    );

    let before = life(&engine, me());
    activate(&mut engine, me(), study, crate::choice::PREPARED_CAST);
    settle(&mut engine, me());
    assert_eq!(life(&engine, me()), before + 1, "the Lesson resolved");
    assert_eq!(
        prepared_markers(&engine, study),
        0,
        "which spent the marker"
    );
    assert!(!offers_prepared_cast(&engine, study));

    activate(&mut engine, me(), study, 1);
    settle(&mut engine, me());
    assert_eq!(
        prepared_markers(&engine, study),
        1,
        "and the effect prepares it anew"
    );
}

// ------------------------------------------------------------- the debt

/// Seat 0 creates the debt on its first turn and is asked for it at its
/// next upkeep — its own, not the opponent's that comes first (CR 603.7a:
/// a delayed trigger created during a resolution).
///
/// `walk_until` panics on any question it was not built to answer, so the
/// opponent's whole turn passing without a `YesNo` is itself the assertion
/// that the debt was not demanded there.
fn demanded(seed: u64) -> Bench {
    let mut engine = bench(
        seed,
        cards(),
        [Seat::with(&[PACT, forest()]), Seat::default()],
    );
    to_main(&mut engine, me());
    let pact = the(&engine, ZoneLocation::Battlefield, PACT);
    activate(&mut engine, me(), pact, 0);
    settle(&mut engine, me());
    let owed = ManaCost::parse("{G}");
    assert!(
        engine.state().delayed.iter().any(|d| {
            d.controller == me()
                && d.when == crate::state::DelayedWhen::NextUpkeep
                && matches!(
                    d.action,
                    crate::state::DelayedAction::PayCostOrLose { cost } if cost == owed
                )
        }),
        "the debt waits for its controller's next upkeep: {:?}",
        engine.state().delayed
    );

    walk_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: crate::choice::YesNoPrompt::PayPact { .. },
                ..
            }
        )
    });
    let turn = &engine.state().turn;
    assert_eq!(
        (turn.number, turn.active, turn.step),
        (3, me(), crate::turn::Step::Upkeep),
        "asked in my next upkeep, having passed through the opponent's"
    );
    let Pending::YesNo {
        player,
        prompt: crate::choice::YesNoPrompt::PayPact { cost },
        ..
    } = engine.pending().clone()
    else {
        unreachable!()
    };
    assert_eq!((player, cost), (me(), owed), "of me, for what I owe");
    engine
}

/// Paid, the game goes on.
#[test]
fn a_debt_paid_at_the_next_upkeep_is_settled() {
    let mut engine = demanded(7143);
    engine.apply(me(), PlayerAction::YesNo(true)).unwrap();
    float_all(&mut engine, me());
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    assert!(!engine.state().players[0].has_lost(), "the debt was paid");
    assert!(
        engine.state().delayed.is_empty(),
        "and nothing more is owed"
    );
}

/// Refused, its controller loses the game.
#[test]
fn a_debt_refused_at_the_next_upkeep_loses_the_game() {
    let mut engine = demanded(7144);
    engine.apply(me(), PlayerAction::YesNo(false)).unwrap();
    assert!(
        engine.state().players[0].has_lost(),
        "\"you lose the game\""
    );
    assert!(!engine.state().players[1].has_lost());
}
