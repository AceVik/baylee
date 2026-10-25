//! Two chosen land types and a timestamp-local toughness floor.

use super::*;
use baylee_cards_dsl::{
    Duration, EnterModifier, Filter, Modifier, PtCount, TargetSpec, static_ability,
};
use baylee_core::{generated::subtypes::land, types::SupertypeSet};

const CHOOSER: u32 = 8610;
const BASIC: u32 = 8611;
const NONBASIC: u32 = 8612;

static CHOICES: &[EnterModifier] = &[EnterModifier::ChooseTwoBasicLandTypes];
static CHANGE: &[AbilityDef] = &[static_ability!(
    Filter::And(&[
        Filter::LAND,
        Filter::HasSupertype(SupertypeSet::BASIC),
        Filter::MatchesChosenTypeOfSource,
    ]),
    Modifier::SetLandTypeToSecondChosen,
)];

#[test]
#[allow(clippy::too_many_lines)] // Both choice sequences share their setup and verify the resulting type, hash, and mana together.
fn two_type_choices_keep_their_order_allow_the_same_type_and_preserve_snow() {
    for destination in [land::ISLAND, land::FOREST] {
        let mut e = bench(
            8610,
            vec![
                card(
                    CHOOSER,
                    FaceDef {
                        enter_modifiers: CHOICES,
                        ..land_face("Choosing land")
                    },
                    KeywordSet::EMPTY,
                    CHANGE,
                ),
                card(
                    BASIC,
                    FaceDef {
                        supertypes: SupertypeSet::BASIC.union(SupertypeSet::SNOW),
                        subtypes: &[land::FOREST],
                        ..land_face("Snow basic forest")
                    },
                    KeywordSet::EMPTY,
                    &[],
                ),
                card(
                    NONBASIC,
                    FaceDef {
                        subtypes: &[land::FOREST],
                        ..land_face("Nonbasic forest")
                    },
                    KeywordSet::EMPTY,
                    &[],
                ),
            ],
            [
                Seat::with(&[BASIC, NONBASIC]).holding(&[CHOOSER]),
                Seat::default(),
            ],
        );
        to_main(&mut e, me());
        let basic = the(&e, ZoneLocation::Battlefield, BASIC);
        let nonbasic = the(&e, ZoneLocation::Battlefield, NONBASIC);
        let chooser = the(&e, ZoneLocation::Hand(me()), CHOOSER);
        e.apply(me(), PlayerAction::PlayLand { card: chooser })
            .unwrap();
        let before_first_choice = e.snapshot_hash();
        for answer in [land::FOREST, destination] {
            let Pending::ChooseSubtype { player, options } = e.pending().clone() else {
                panic!("expected a land-type choice, got {:?}", e.pending());
            };
            assert_eq!(player, me());
            assert_eq!(
                options,
                [
                    land::PLAINS,
                    land::ISLAND,
                    land::SWAMP,
                    land::MOUNTAIN,
                    land::FOREST
                ]
            );
            let illegal = PlayerAction::ChooseSubtype(land::DESERT);
            assert!(e.pending().answer_fault(&illegal).is_some());
            assert!(e.apply(me(), illegal).is_err());
            e.apply(me(), PlayerAction::ChooseSubtype(answer)).unwrap();
        }
        assert_ne!(before_first_choice, e.snapshot_hash());
        let mut other_second_choice = e.state().clone();
        for rider in &mut other_second_choice.object_mut(chooser).unwrap().riders {
            if let crate::object::Rider::SecondChosenSubtype(subtype) = rider {
                *subtype = land::SWAMP;
            }
        }
        assert_ne!(
            e.state().snapshot_hash(),
            other_second_choice.snapshot_hash()
        );
        assert_ne!(
            e.state().loop_signature(),
            other_second_choice.loop_signature()
        );
        let result = e.state().object(basic).unwrap().characteristics();
        assert!(result.subtypes.contains(destination));
        assert_eq!(
            result.subtypes.contains(land::FOREST),
            destination == land::FOREST
        );
        assert!(result.supertypes.contains(SupertypeSet::SNOW));
        assert!(
            e.state()
                .object(nonbasic)
                .unwrap()
                .characteristics()
                .subtypes
                .contains(land::FOREST)
        );
        let (_, legal) = offer(&e);
        assert!(legal.mana_abilities.contains(&basic));
        e.apply(me(), PlayerAction::ActivateManaAbility { source: basic })
            .unwrap();
        let color = if destination == land::ISLAND {
            baylee_core::mana::ManaColor::Blue
        } else {
            baylee_core::mana::ManaColor::Green
        };
        assert_eq!(e.state().players[0].mana_pool.available(color), 1);
    }
}

const REDUCER: u32 = 8613;
const CREATURE: u32 = 8614;
static SNOW: Filter = Filter::And(&[Filter::LAND, Filter::HasSupertype(SupertypeSet::SNOW)]);
static REDUCE: &[AbilityDef] = &[
    free(
        &[Effect::continuous(
            &Filter::This,
            Modifier::ReducePTByCount {
                count: PtCount::ControllerOrDefenderControls(&SNOW),
                minimum_toughness: Some(1),
            },
            Duration::Indefinitely,
        )],
        Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    ),
    free(
        &[Effect::continuous(
            &Filter::This,
            Modifier::ModifyPT(0, -1),
            Duration::UntilEndOfTurn,
        )],
        Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    ),
    free(
        &[Effect::Destroy {
            target: TargetSpec::Object(&Filter::LAND),
            no_regen: false,
        }],
        Some(TargetReq::one(TargetSpec::Object(&Filter::LAND))),
    ),
    free(
        &[Effect::AddCounter {
            kind: baylee_cards_dsl::CounterKind::P1P1,
            amount: baylee_cards_dsl::Amount::Fixed(1),
        }],
        Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    ),
];

fn target(e: &mut Bench, object: ObjectId) {
    let Pending::ChooseTargets { player, .. } = e.pending().clone() else {
        panic!("expected targets");
    };
    e.apply(
        player,
        PlayerAction::ChooseTargets {
            objects: vec![object],
            players: vec![],
        },
    )
    .unwrap();
}

fn pt(e: &Bench, object: ObjectId) -> (Option<i16>, Option<i16>) {
    let c = e.state().object(object).unwrap().characteristics();
    (c.power, c.toughness)
}

#[test]
fn reductions_count_the_affected_controller_or_actual_defender_and_only_bound_their_own_timestamp()
{
    let third = PlayerId::new(2);
    let mut e = bench(
        8613,
        vec![
            card(
                REDUCER,
                face("Reducer", "{0}", TypeSet::ARTIFACT),
                KeywordSet::EMPTY,
                REDUCE,
            ),
            card(
                CREATURE,
                creature_face("Attacker", "{0}", 5, 3),
                KeywordSet::EMPTY,
                &[],
            ),
            card(
                BASIC,
                FaceDef {
                    supertypes: SupertypeSet::SNOW,
                    subtypes: &[land::FOREST],
                    ..land_face("Snow forest")
                },
                KeywordSet::EMPTY,
                &[],
            ),
        ],
        [
            Seat::with(&[REDUCER]),
            Seat::with(&[CREATURE, BASIC, BASIC]),
            Seat::with(&[BASIC, BASIC, BASIC, BASIC]),
        ],
    );
    to_main(&mut e, me());
    let reducer = the(&e, ZoneLocation::Battlefield, REDUCER);
    let creature = the(&e, ZoneLocation::Battlefield, CREATURE);
    assert_eq!(pt(&e, creature), (Some(5), Some(3)));
    activate(&mut e, me(), reducer, 0);
    target(&mut e, creature);
    settle(&mut e, me());
    assert_eq!(
        pt(&e, creature),
        (Some(3), Some(1)),
        "the affected controller has two snow lands; the effect controller has none"
    );
    to_main(&mut e, them());
    walk_until(
        &mut e,
        |game| matches!(game.pending(), Pending::ChooseAttackers { player, .. } if *player == them()),
    );
    e.apply(
        them(),
        PlayerAction::DeclareAttackers {
            attackers: vec![(creature, baylee_core::ids::Defender::Player(third))],
        },
    )
    .unwrap();
    assert_eq!(
        pt(&e, creature),
        (Some(1), Some(1)),
        "the actual defending seat has four snow lands"
    );
    settle(&mut e, me());
    let snow = objects(&e, ZoneLocation::Battlefield, BASIC)
        .into_iter()
        .find(|id| e.state().object(*id).unwrap().controller == third)
        .unwrap();
    activate(&mut e, me(), reducer, 2);
    target(&mut e, snow);
    settle(&mut e, me());
    assert_eq!(
        pt(&e, creature),
        (Some(2), Some(1)),
        "a real land departure updates the count during combat"
    );
    activate(&mut e, me(), reducer, 1);
    target(&mut e, creature);
    settle(&mut e, me());
    assert_eq!(
        e.state().object(creature).unwrap().zone,
        crate::zone::Zone::Graveyard,
        "a later reduction can make toughness zero"
    );
}

#[test]
fn pt_counter_placement_restamps_the_kind_but_removal_keeps_its_order_against_a_floor() {
    static REMOVE: &[AbilityDef] = &[free(
        &[Effect::RemoveCounterSelf {
            kind: baylee_cards_dsl::CounterKind::P1P1,
            n: 1,
        }],
        None,
    )];
    let mut e = bench(
        8615,
        vec![
            card(
                REDUCER,
                face("Reducer", "{0}", TypeSet::ARTIFACT),
                KeywordSet::EMPTY,
                REDUCE,
            ),
            card(
                CREATURE,
                creature_face("Counter bearer", "{0}", 1, 1),
                KeywordSet::EMPTY,
                REMOVE,
            ),
            card(
                BASIC,
                FaceDef {
                    supertypes: SupertypeSet::SNOW,
                    subtypes: &[land::FOREST],
                    ..land_face("Snow forest")
                },
                KeywordSet::EMPTY,
                &[],
            ),
        ],
        [
            Seat::with(&[REDUCER, CREATURE, BASIC, BASIC, BASIC, BASIC]),
            Seat::default(),
        ],
    );
    to_main(&mut e, me());
    let reducer = the(&e, ZoneLocation::Battlefield, REDUCER);
    let creature = the(&e, ZoneLocation::Battlefield, CREATURE);
    for _ in 0..2 {
        activate(&mut e, me(), reducer, 3);
        target(&mut e, creature);
        settle(&mut e, me());
    }
    assert_eq!(pt(&e, creature), (Some(3), Some(3)));
    activate(&mut e, me(), reducer, 0);
    target(&mut e, creature);
    settle(&mut e, me());
    assert_eq!(
        pt(&e, creature),
        (Some(-1), Some(1)),
        "older counters apply before the bounded reduction"
    );
    activate(&mut e, me(), creature, 0);
    settle(&mut e, me());
    assert_eq!(
        pt(&e, creature),
        (Some(-2), Some(1)),
        "removing a counter does not move its kind after the reduction"
    );
    activate(&mut e, me(), reducer, 3);
    target(&mut e, creature);
    settle(&mut e, me());
    assert_eq!(
        pt(&e, creature),
        (Some(-1), Some(3)),
        "a new counter restamps both counters of its kind after the reduction"
    );
    for _ in 0..3 {
        activate(&mut e, me(), reducer, 1);
        target(&mut e, creature);
        settle(&mut e, me());
    }
    assert_eq!(
        e.state().object(creature).unwrap().zone,
        crate::zone::Zone::Graveyard
    );
}
