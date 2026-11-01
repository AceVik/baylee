//! Choices made during resolution belong to that resolution even after
//! its source leaves (CR 113.7a, 608.2d).

use super::*;
use baylee_cards_dsl::{Amount, Condition, Filter, PlayerRel, TargetSpec, ThisWay};

const NAMER: u32 = 7403;
const NAMED: u32 = 7404;

const fn choose_and_shoot(remember: bool) -> [Effect; 3] {
    [
        Effect::ChooseCardName {
            player: PlayerRel::You,
            types: TypeSet::EMPTY,
            excluded_types: TypeSet::EMPTY,
            remember,
        },
        Effect::RevealRandomFromHand {
            who: PlayerRel::You,
            count: Amount::Fixed(1),
        },
        Effect::IfCondition {
            condition: Condition::AmountAtLeast(
                Amount::ThisWay(ThisWay::Revealed(&Filter::NamedBySource)),
                1,
            ),
            then: &[Effect::DealDamage {
                target: TargetSpec::AnyTarget,
                amount: Amount::Fixed(2),
            }],
            otherwise: &[],
        },
    ]
}

static REMEMBERED: &[Effect] = &choose_and_shoot(true);
static LOCAL: &[Effect] = &choose_and_shoot(false);
static ABILITIES: &[AbilityDef] = &[
    free(REMEMBERED, Some(TargetReq::one(TargetSpec::AnyTarget))),
    free(LOCAL, Some(TargetReq::one(TargetSpec::AnyTarget))),
    free(&[Effect::SacrificeSelf], None),
    free(
        &[Effect::blink_to_owner(TargetSpec::Object(
            &Filter::ARTIFACT,
        ))],
        Some(TargetReq::one(TargetSpec::Object(&Filter::ARTIFACT))),
    ),
];

fn fixture() -> (Bench, ObjectId) {
    let mut engine = bench(
        7403,
        vec![
            card(
                NAMER,
                face("Namer", "{0}", TypeSet::ARTIFACT),
                KeywordSet::EMPTY,
                ABILITIES,
            ),
            card(
                NAMED,
                creature_face("Named", "{0}", 2, 2),
                KeywordSet::EMPTY,
                &[],
            ),
        ],
        [Seat::with(&[NAMER]).holding(&[NAMED]), Seat::default()],
    );
    to_main(&mut engine, me());
    let source = the(&engine, ZoneLocation::Battlefield, NAMER);
    assert_eq!(engine.state().zones.list(ZoneLocation::Hand(me())).len(), 1);
    (engine, source)
}

fn shoot(engine: &mut Bench, source: ObjectId, ability: u32, departure: Option<u32>, chosen: u32) {
    let version = engine.state().object(source).unwrap().version;
    activate(engine, me(), source, ability);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![them()],
            },
        )
        .unwrap();
    if let Some(departure) = departure {
        activate(engine, me(), source, departure);
        if departure == 3 {
            engine
                .apply(
                    me(),
                    PlayerAction::ChooseTargets {
                        objects: vec![source],
                        players: vec![],
                    },
                )
                .unwrap();
        }
    }
    walk_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseCardName { .. })
    });
    if departure.is_some() {
        assert!(engine.state().object(source).unwrap().version > version);
    }
    engine
        .apply(
            me(),
            PlayerAction::ChooseCardName {
                card: CardIndex::new(chosen),
                face: 0,
            },
        )
        .unwrap();
    settle(engine, me());
}

#[test]
fn a_resolution_local_name_matches_without_persisting_on_the_source() {
    for ability in [0, 1] {
        for (chosen, expected) in [(NAMED, 18), (NAMER, 20)] {
            let (mut engine, source) = fixture();
            shoot(&mut engine, source, ability, None, chosen);
            assert_eq!(life(&engine, them()), expected);
        }
    }
}

#[test]
fn a_name_chosen_after_the_source_departed_still_gates_the_effect() {
    for ability in [0, 1] {
        for departure in [2, 3] {
            for (chosen, expected) in [(NAMED, 18), (NAMER, 20)] {
                let (mut engine, source) = fixture();
                shoot(&mut engine, source, ability, Some(departure), chosen);
                assert_eq!(life(&engine, them()), expected);
                if departure == 3 {
                    assert!(
                        engine.state().object(source).unwrap().chosen_name.is_none(),
                        "the returned source does not inherit the old ability's choice"
                    );
                }
            }
        }
    }
}

#[test]
fn the_current_choice_overrides_a_different_name_remembered_by_the_source() {
    let (mut engine, source) = fixture();
    shoot(&mut engine, source, 0, None, NAMED);
    assert_eq!(life(&engine, them()), 18);
    shoot(&mut engine, source, 1, None, NAMER);
    assert_eq!(
        life(&engine, them()),
        18,
        "the older matching name must not replace this resolution's mismatch"
    );
}
