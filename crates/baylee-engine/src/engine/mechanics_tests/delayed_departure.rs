//! A dies trigger can remember its card's graveyard incarnation for a
//! delayed return (CR 400.7e, 603.7c), but never a later incarnation.

use super::*;
use baylee_cards_dsl::{Amount, Filter, PlayerRel, TargetSpec, Trigger};

const RETURNER: u32 = 7398;
const RECYCLER: u32 = 7399;

static RETURN_LATER: &[Effect] = &[
    Effect::LoseLife {
        amount: Amount::Fixed(1),
        target: PlayerRel::You,
    },
    Effect::ReturnToHand {
        target: TargetSpec::EventObject,
    },
];
static RETURNER_ABILITIES: &[AbilityDef] = &[
    free(&[Effect::SacrificeSelf], None),
    baylee_cards_dsl::triggered!(
        Trigger::Dies(&Filter::This),
        &[Effect::AtNextEndStep {
            effects: RETURN_LATER
        },]
    ),
];
static RECYCLE: &[AbilityDef] = &[free(
    &[
        Effect::ReturnToHand {
            target: TargetSpec::CardInGraveyard(&Filter::CREATURE, PlayerRel::You),
        },
        Effect::DiscardHand {
            who: PlayerRel::You,
        },
    ],
    Some(TargetReq::one(TargetSpec::CardInGraveyard(
        &Filter::CREATURE,
        PlayerRel::You,
    ))),
)];

fn fixture() -> (Bench, ObjectId, ObjectId) {
    let mut engine = bench(
        7398,
        vec![
            card(
                RETURNER,
                creature_face("Returner", "{0}", 1, 1),
                KeywordSet::EMPTY,
                RETURNER_ABILITIES,
            ),
            card(
                RECYCLER,
                face("Recycler", "{0}", TypeSet::ARTIFACT),
                KeywordSet::EMPTY,
                RECYCLE,
            ),
        ],
        [Seat::with(&[RETURNER, RECYCLER]), Seat::default()],
    );
    to_main(&mut engine, me());
    let returner = the(&engine, ZoneLocation::Battlefield, RETURNER);
    let recycler = the(&engine, ZoneLocation::Battlefield, RECYCLER);
    (engine, returner, recycler)
}

fn recycle(engine: &mut Bench, recycler: ObjectId, returner: ObjectId) {
    activate(engine, me(), recycler, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![returner],
                players: vec![],
            },
        )
        .unwrap();
    settle(engine, me());
}

fn reach_after_end(engine: &mut Bench) {
    walk_until(engine, |e| {
        e.state().turn.step == Step::End && e.state().zones.stack_is_empty()
    });
}

#[test]
fn dies_trigger_delays_return_of_its_graveyard_successor_until_end_step() {
    let (mut engine, returner, _) = fixture();
    activate(&mut engine, me(), returner, 0);
    settle(&mut engine, me());
    assert_eq!(
        the(&engine, ZoneLocation::Graveyard(me()), RETURNER),
        returner
    );
    assert_eq!(life(&engine, me()), 20, "the life loss also waits");
    reach_after_end(&mut engine);
    assert_eq!(the(&engine, ZoneLocation::Hand(me()), RETURNER), returner);
    assert_eq!(life(&engine, me()), 19);
}

#[test]
fn delayed_return_does_not_follow_a_card_out_of_the_graveyard_and_back() {
    let (mut engine, returner, recycler) = fixture();
    activate(&mut engine, me(), returner, 0);
    settle(&mut engine, me());
    recycle(&mut engine, recycler, returner);
    assert_eq!(
        the(&engine, ZoneLocation::Graveyard(me()), RETURNER),
        returner
    );
    reach_after_end(&mut engine);
    assert_eq!(
        the(&engine, ZoneLocation::Graveyard(me()), RETURNER),
        returner
    );
    assert!(objects(&engine, ZoneLocation::Hand(me()), RETURNER).is_empty());
    assert_eq!(life(&engine, me()), 19, "only the return loses its subject");
}

#[test]
fn departure_successor_is_not_recaptured_after_leaving_before_the_dies_trigger_resolves() {
    let (mut engine, returner, recycler) = fixture();
    activate(&mut engine, me(), returner, 0);
    walk_until(&mut engine, |e| {
        !objects(e, ZoneLocation::Graveyard(me()), RETURNER).is_empty()
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == me())
    });
    // The dies trigger is on the stack. This activation takes its card out
    // of the graveyard and back before the delayed trigger is created.
    recycle(&mut engine, recycler, returner);
    reach_after_end(&mut engine);
    assert_eq!(
        the(&engine, ZoneLocation::Graveyard(me()), RETURNER),
        returner
    );
    assert!(objects(&engine, ZoneLocation::Hand(me()), RETURNER).is_empty());
    assert_eq!(life(&engine, me()), 19);
}
