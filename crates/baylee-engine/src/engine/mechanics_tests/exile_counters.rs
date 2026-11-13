//! Exile-after-death identity and counters removed from a chosen exiled card.
use super::*;
use baylee_cards_dsl::{Amount, Filter, PlayerRel, TargetSpec, Trigger, counters, triggered};

const WATCHER: u32 = 99_650;
const VICTIM: u32 = 99_651;
const OWN_VICTIM: u32 = 99_652;
const FIRST: u32 = 99_653;
const SECOND: u32 = 99_654;
const INSUFFICIENT: u32 = 99_655;
const THEIRS: u32 = 99_656;

static EXILE_DEAD: &[Effect] = &[Effect::ExileEventFromGraveyardWithCounters {
    kind: counters::HIT,
    n: 1,
}];
static DEATH_WATCH: &[AbilityDef] = &[triggered!(
    Trigger::Dies(&Filter::OPPONENT_CREATURE),
    EXILE_DEAD
)];
static DAMAGE_DEATH_WATCH: &[AbilityDef] = &[
    free(
        &[Effect::DealDamage {
            amount: Amount::Fixed(1),
            target: TargetSpec::Object(&Filter::CREATURE),
        }],
        Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    ),
    triggered!(
        Trigger::DiesAfterDamageByThis(&Filter::OPPONENT_CREATURE),
        EXILE_DEAD
    ),
];
static PAYMENT: &[Effect] = &[Effect::MayDo {
    effects: &[
        Effect::RemoveCountersFromChosenExile {
            owner: PlayerRel::You,
            kind: counters::HIT,
            n: 2,
        },
        Effect::GainLife {
            amount: Amount::Fixed(4),
        },
    ],
}];
static PAY_HIT: &[AbilityDef] = &[free(PAYMENT, None)];

fn creature(index: u32, name: &'static str, abilities: &'static [AbilityDef]) -> &'static CardDef {
    card(
        index,
        creature_face(name, "{0}", 1, 1),
        KeywordSet::EMPTY,
        abilities,
    )
}

fn counters(engine: &Bench, object: ObjectId) -> u16 {
    engine
        .state
        .object(object)
        .unwrap()
        .counters
        .get(counters::HIT)
}

fn move_to(engine: &mut Bench, object: ObjectId, to: ZoneLocation) {
    engine
        .state
        .move_object(object, to, ZonePosition::Top, Cause::Effect)
        .unwrap();
}

fn destroy_and_collect(engine: &mut Bench, victim: ObjectId) {
    crate::sba::destroy(&mut engine.state, victim);
    let (player, _) = offer(engine);
    engine.apply(player, PlayerAction::PassPriority).unwrap();
}

#[test]
fn death_exiles_only_an_opponents_creature_and_marks_its_graveyard_card() {
    let watcher = creature(WATCHER, "Death watcher", DEATH_WATCH);
    let victim = creature(VICTIM, "Opposing victim", &[]);
    let own_victim = creature(OWN_VICTIM, "Friendly victim", &[]);
    let mut engine = bench(
        99_650,
        vec![watcher, victim, own_victim],
        [Seat::with(&[WATCHER, OWN_VICTIM]), Seat::with(&[VICTIM])],
    );
    to_main(&mut engine, me());
    let watcher = the(&engine, ZoneLocation::Battlefield, WATCHER);
    let ours = the(&engine, ZoneLocation::Battlefield, OWN_VICTIM);
    let theirs = the(&engine, ZoneLocation::Battlefield, VICTIM);

    destroy_and_collect(&mut engine, ours);
    settle(&mut engine, me());
    assert_eq!(engine.state.object(ours).unwrap().zone, Zone::Graveyard);
    assert_eq!(counters(&engine, ours), 0);
    assert_eq!(triggered(&engine, watcher), 0);

    destroy_and_collect(&mut engine, theirs);
    settle(&mut engine, me());
    assert_eq!(engine.state.object(theirs).unwrap().zone, Zone::Exile);
    assert_eq!(counters(&engine, theirs), 1);
    assert_eq!(triggered(&engine, watcher), 1);
}

#[test]
fn a_death_cannot_follow_a_card_that_left_and_returned_before_resolution() {
    let watcher = creature(WATCHER, "Death watcher", DEATH_WATCH);
    let victim = creature(VICTIM, "Opposing victim", &[]);
    let mut engine = bench(
        99_651,
        vec![watcher, victim],
        [Seat::with(&[WATCHER]), Seat::with(&[VICTIM])],
    );
    to_main(&mut engine, me());
    let theirs = the(&engine, ZoneLocation::Battlefield, VICTIM);
    destroy_and_collect(&mut engine, theirs);
    assert!(!engine.state.zones.stack_is_empty(), "the death triggered");

    move_to(&mut engine, theirs, ZoneLocation::Exile(them()));
    move_to(&mut engine, theirs, ZoneLocation::Graveyard(them()));
    settle(&mut engine, me());

    assert_eq!(engine.state.object(theirs).unwrap().zone, Zone::Graveyard);
    assert_eq!(counters(&engine, theirs), 0);
    assert!(
        engine
            .state
            .zones
            .list(ZoneLocation::Exile(them()))
            .is_empty()
    );
}

#[test]
fn a_later_death_follows_the_battlefield_identity_captured_by_damage_history() {
    let watcher = creature(WATCHER, "Damage watcher", DAMAGE_DEATH_WATCH);
    let victim = creature(VICTIM, "Opposing victim", &[]);
    let mut engine = bench(
        99_652,
        vec![watcher, victim],
        [Seat::with(&[WATCHER]), Seat::with(&[VICTIM])],
    );
    to_main(&mut engine, me());
    let watcher = the(&engine, ZoneLocation::Battlefield, WATCHER);
    let theirs = the(&engine, ZoneLocation::Battlefield, VICTIM);
    activate(&mut engine, me(), watcher, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![theirs],
                players: vec![],
            },
        )
        .unwrap();
    settle(&mut engine, me());

    assert_eq!(engine.state.object(theirs).unwrap().zone, Zone::Exile);
    assert_eq!(counters(&engine, theirs), 1);
}

fn payment_game(seed: u64) -> Bench {
    let watcher = creature(WATCHER, "Hit payer", PAY_HIT);
    let first = creature(FIRST, "First marked card", &[]);
    let second = creature(SECOND, "Second marked card", &[]);
    let insufficient = creature(INSUFFICIENT, "Insufficiently marked card", &[]);
    let theirs = creature(THEIRS, "Opponents marked card", &[]);
    let mut engine = bench(
        seed,
        vec![watcher, first, second, insufficient, theirs],
        [
            Seat::with(&[WATCHER, FIRST, SECOND, INSUFFICIENT]),
            Seat::with(&[THEIRS]),
        ],
    );
    to_main(&mut engine, me());
    engine
}

fn begin_payment(engine: &mut Bench) {
    let watcher = the(engine, ZoneLocation::Battlefield, WATCHER);
    activate(engine, me(), watcher, 0);
    walk_until(engine, |e| matches!(e.pending(), Pending::YesNo { .. }));
    engine.apply(me(), PlayerAction::YesNo(true)).unwrap();
}

#[test]
fn exile_choices_use_seat_order_and_card_ownership_not_the_exile_pile() {
    let mut engine = payment_game(99_653);
    let first = the(&engine, ZoneLocation::Battlefield, FIRST);
    let second = the(&engine, ZoneLocation::Battlefield, SECOND);
    let insufficient = the(&engine, ZoneLocation::Battlefield, INSUFFICIENT);
    let theirs = the(&engine, ZoneLocation::Battlefield, THEIRS);
    move_to(&mut engine, first, ZoneLocation::Exile(me()));
    move_to(&mut engine, second, ZoneLocation::Exile(them()));
    move_to(&mut engine, insufficient, ZoneLocation::Exile(me()));
    move_to(&mut engine, theirs, ZoneLocation::Exile(me()));
    for (card, n) in [(first, 3), (second, 2), (insufficient, 1), (theirs, 2)] {
        crate::replacement::put_counters(&mut engine.state, card, counters::HIT, n);
    }
    let before_life = life(&engine, me());

    begin_payment(&mut engine);
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("a marked card must be chosen: {:?}", engine.pending())
    };
    assert_eq!(options, vec![first, second]);
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![second],
            },
        )
        .unwrap();
    settle(&mut engine, me());

    assert_eq!(counters(&engine, first), 3);
    assert_eq!(counters(&engine, second), 0);
    assert_eq!(counters(&engine, insufficient), 1);
    assert_eq!(counters(&engine, theirs), 2);
    assert_eq!(life(&engine, me()), before_life + 4);
}

#[test]
fn a_stale_exile_choice_is_refused_before_its_reward_can_resolve() {
    for moved_away in [false, true] {
        let mut engine = payment_game(99_654);
        let marked = the(&engine, ZoneLocation::Battlefield, FIRST);
        move_to(&mut engine, marked, ZoneLocation::Exile(them()));
        crate::replacement::put_counters(&mut engine.state, marked, counters::HIT, 2);
        let before_life = life(&engine, me());
        begin_payment(&mut engine);

        if moved_away {
            move_to(&mut engine, marked, ZoneLocation::Graveyard(me()));
        } else {
            assert_eq!(
                crate::replacement::remove_counters(&mut engine.state, marked, counters::HIT, 1),
                1
            );
        }
        assert!(
            engine
                .apply(
                    me(),
                    PlayerAction::ChooseObjects {
                        objects: vec![marked]
                    }
                )
                .is_err(),
            "moved_away={moved_away}: the old answer is no longer legal"
        );
        assert_eq!(
            life(&engine, me()),
            before_life,
            "no reward without removal"
        );
        assert!(matches!(engine.pending(), Pending::ChooseCards { .. }));

        if !moved_away {
            crate::replacement::put_counters(&mut engine.state, marked, counters::HIT, 1);
            engine
                .apply(
                    me(),
                    PlayerAction::ChooseObjects {
                        objects: vec![marked],
                    },
                )
                .unwrap();
            settle(&mut engine, me());
            assert_eq!(counters(&engine, marked), 0);
            assert_eq!(life(&engine, me()), before_life + 4);
        }
    }
}
