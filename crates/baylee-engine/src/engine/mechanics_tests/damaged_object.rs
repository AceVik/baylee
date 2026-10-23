//! Damage received by a permanent retains both its amount and its exact
//! recipient. Several combat sources are one event for that recipient.

use super::*;
use baylee_cards_dsl::{Amount, Filter, PlayerRel, TargetSpec, Trigger};
use baylee_core::ids::Defender;

const WATCHER: u32 = 7392;
const VICTIM: u32 = 7393;
const TWO: u32 = 7394;
const THREE: u32 = 7395;
const PINGER: u32 = 7396;
const REFLECTOR: u32 = 7397;

static WATCH: &[AbilityDef] = &[baylee_cards_dsl::triggered!(
    Trigger::DealtDamage(&Filter::CREATURE),
    &[
        Effect::GainLife {
            amount: Amount::EventAmount
        },
        Effect::ReturnToHand {
            target: TargetSpec::EventObject
        },
    ]
)];
static REFLECT: &[AbilityDef] = &[baylee_cards_dsl::triggered!(
    Trigger::DealtDamage(&Filter::This),
    &[Effect::DealDamage {
        amount: Amount::EventAmount,
        target: TargetSpec::Player(PlayerRel::You),
    }]
)];
static PING: &[AbilityDef] = &[free(
    &[Effect::DealDamage {
        amount: Amount::Fixed(3),
        target: TargetSpec::Object(&Filter::CREATURE),
    }],
    Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
)];

fn cards() -> Vec<&'static CardDef> {
    vec![
        card(
            WATCHER,
            face("Watcher", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            WATCH,
        ),
        card(
            VICTIM,
            creature_face("Victim", "{0}", 0, 12),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            TWO,
            creature_face("Two", "{0}", 2, 12),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            THREE,
            creature_face("Three", "{0}", 3, 12),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            PINGER,
            face("Pinger", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            PING,
        ),
        card(
            REFLECTOR,
            creature_face("Reflector", "{0}", 0, 2),
            KeywordSet::EMPTY,
            REFLECT,
        ),
    ]
}

fn ping(engine: &mut Bench, source: ObjectId, target: ObjectId) {
    activate(engine, me(), source, 0);
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

#[test]
fn received_combat_damage_totals_sources_and_binds_only_the_recipient() {
    let mut engine = bench(
        7392,
        cards(),
        [Seat::with(&[WATCHER, VICTIM]), Seat::with(&[TWO, THREE])],
    );
    let victim = the(&engine, ZoneLocation::Battlefield, VICTIM);
    let watcher = the(&engine, ZoneLocation::Battlefield, WATCHER);
    let two = the(&engine, ZoneLocation::Battlefield, TWO);
    let three = the(&engine, ZoneLocation::Battlefield, THREE);
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            me(),
            PlayerAction::DeclareAttackers {
                attackers: vec![(victim, Defender::Player(them()))],
            },
        )
        .unwrap();
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            them(),
            PlayerAction::DeclareBlockers {
                blockers: vec![(two, victim), (three, victim)],
            },
        )
        .unwrap();
    walk_until(&mut engine, |e| e.state().turn.step == Step::CombatEnd);
    assert_eq!(
        life(&engine, me()),
        25,
        "both blockers' damage belongs to one event"
    );
    assert_eq!(triggered(&engine, watcher), 1);
    assert_eq!(the(&engine, ZoneLocation::Hand(me()), VICTIM), victim);
    for id in [watcher, two, three] {
        assert_eq!(
            engine.state().object(id).unwrap().zone,
            crate::zone::Zone::Battlefield
        );
    }
}

#[test]
fn received_noncombat_damage_binds_the_hit_creature_and_leaves_its_neighbor() {
    let mut engine = bench(
        7393,
        cards(),
        [
            Seat::with(&[WATCHER, PINGER]),
            Seat::with(&[VICTIM, VICTIM]),
        ],
    );
    to_main(&mut engine, me());
    let victims = objects(&engine, ZoneLocation::Battlefield, VICTIM);
    let pinger = the(&engine, ZoneLocation::Battlefield, PINGER);
    let watcher = the(&engine, ZoneLocation::Battlefield, WATCHER);
    ping(&mut engine, pinger, victims[0]);
    assert_eq!(life(&engine, me()), 23);
    assert_eq!(the(&engine, ZoneLocation::Hand(them()), VICTIM), victims[0]);
    assert_eq!(the(&engine, ZoneLocation::Battlefield, VICTIM), victims[1]);
    assert_eq!(triggered(&engine, watcher), 1);
}

#[test]
fn lethal_damage_retains_its_amount_after_the_receivers_death() {
    let mut engine = bench(
        7394,
        cards(),
        [Seat::with(&[REFLECTOR, PINGER]), Seat::default()],
    );
    to_main(&mut engine, me());
    let reflector = the(&engine, ZoneLocation::Battlefield, REFLECTOR);
    let pinger = the(&engine, ZoneLocation::Battlefield, PINGER);
    ping(&mut engine, pinger, reflector);
    assert_eq!(
        life(&engine, me()),
        17,
        "the lethal hit's full amount is retained"
    );
    assert_eq!(
        the(&engine, ZoneLocation::Graveyard(me()), REFLECTOR),
        reflector
    );
    assert_eq!(triggered(&engine, reflector), 1);
}
