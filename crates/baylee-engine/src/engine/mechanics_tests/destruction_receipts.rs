//! Follow-up instructions count destruction, including redirected destinations.

use super::*;
use crate::choice::YesNoPrompt;
use baylee_cards_dsl::{Amount, Duration, Filter, Modifier, PlayerRel, TargetSpec, static_ability};
use baylee_core::generated::subtypes::land;

const SOURCE: u32 = 8620;
const ORDINARY: u32 = 8621;
const EXILED: u32 = 8622;
const INDESTRUCTIBLE: u32 = 8623;
const REGENERATING: u32 = 8624;
const STOLEN: u32 = 8625;
static PLAINS: Filter = Filter::And(&[Filter::LAND, Filter::HasSubtype(land::PLAINS)]);
static TAX: &[Effect] = &[Effect::PlayerMayPayOr {
    player: PlayerRel::EventPlayer,
    mana: Amount::Fixed(2),
    effect: &Effect::DealDamage {
        amount: Amount::Fixed(1),
        target: TargetSpec::Player(PlayerRel::EventPlayer),
    },
}];
static SOURCE_ABILITIES: &[AbilityDef] = &[
    free(
        &[Effect::DestroyAllThenForEachDestroyed {
            filter: &PLAINS,
            no_regen: false,
            effects: TAX,
        }],
        None,
    ),
    free(
        &[Effect::continuous(
            &Filter::This,
            Modifier::GainControl,
            Duration::Indefinitely,
        )],
        Some(TargetReq::one(TargetSpec::Object(&Filter::LAND))),
    ),
    free(
        &[Effect::AddCounter {
            kind: baylee_cards_dsl::CounterKind::Shield,
            amount: Amount::Fixed(1),
        }],
        Some(TargetReq::one(TargetSpec::Object(&Filter::LAND))),
    ),
];
static EXILE: &[AbilityDef] = &[static_ability!(
    Filter::This,
    Modifier::ExileInsteadOfLeavingBattlefield
)];
static REGENERATE: &[AbilityDef] = &[free(
    &[Effect::Regenerate {
        target: TargetSpec::ThisObject,
    }],
    None,
)];

fn plains(name: &'static str) -> FaceDef {
    FaceDef {
        subtypes: &[land::PLAINS],
        ..land_face(name)
    }
}

fn walk_until(e: &mut Bench, done: impl Fn(&Bench) -> bool) {
    for _ in 0..600 {
        if done(e) {
            return;
        }
        let pending = e.pending().clone();
        if let Pending::Arrange {
            player,
            prompt: crate::choice::ArrangePrompt::Order,
            ..
        } = pending
        {
            e.apply(
                player,
                crate::choice::timeout_answer(&pending).expect("order answer"),
            )
            .unwrap();
        } else {
            assert!(walk_past(e, &pending), "unexpected question: {pending:?}");
        }
    }
    panic!("never reached expected question");
}

#[test]
#[allow(clippy::too_many_lines)] // One sweep scenario tracks replacements, original controllers, and every payment together.
fn a_sweep_finishes_before_per_object_payments_and_retains_destroyed_objects_controllers() {
    let mut e = bench(
        8620,
        vec![
            card(
                SOURCE,
                plains("Sweeping land"),
                KeywordSet::EMPTY,
                SOURCE_ABILITIES,
            ),
            card(ORDINARY, plains("Ordinary plains"), KeywordSet::EMPTY, &[]),
            card(STOLEN, plains("Borrowed plains"), KeywordSet::EMPTY, &[]),
            card(EXILED, plains("Exiling plains"), KeywordSet::EMPTY, EXILE),
            card(
                INDESTRUCTIBLE,
                plains("Indestructible plains"),
                KeywordSet::INDESTRUCTIBLE,
                &[],
            ),
            card(
                REGENERATING,
                plains("Regenerating plains"),
                KeywordSet::EMPTY,
                REGENERATE,
            ),
        ],
        [
            Seat::with(&[SOURCE, EXILED, forest(), forest()]),
            Seat::with(&[ORDINARY, STOLEN, INDESTRUCTIBLE, REGENERATING]),
        ],
    );
    to_main(&mut e, me());
    let source = the(&e, ZoneLocation::Battlefield, SOURCE);
    let stolen = the(&e, ZoneLocation::Battlefield, STOLEN);
    let ordinary = the(&e, ZoneLocation::Battlefield, ORDINARY);
    let exiled = the(&e, ZoneLocation::Battlefield, EXILED);
    let indestructible = the(&e, ZoneLocation::Battlefield, INDESTRUCTIBLE);
    let regenerating = the(&e, ZoneLocation::Battlefield, REGENERATING);
    activate(&mut e, me(), source, 1);
    e.apply(
        me(),
        PlayerAction::ChooseTargets {
            objects: vec![stolen],
            players: vec![],
        },
    )
    .unwrap();
    settle(&mut e, me());
    assert_eq!(e.state().object(stolen).unwrap().owner, them());
    assert_eq!(e.state().object(stolen).unwrap().controller, me());
    activate(&mut e, me(), source, 2);
    e.apply(
        me(),
        PlayerAction::ChooseTargets {
            objects: vec![regenerating],
            players: vec![],
        },
    )
    .unwrap();
    settle(&mut e, me());
    e.apply(me(), PlayerAction::PassPriority).unwrap();
    activate(&mut e, them(), regenerating, 0);
    settle(&mut e, me());
    // Two surviving Forests float the one payment; the destroyed Plains
    // cannot be tapped after the first payment question appears.
    for id in objects(&e, ZoneLocation::Battlefield, forest()) {
        e.apply(me(), PlayerAction::ActivateManaAbility { source: id })
            .unwrap();
    }
    activate(&mut e, me(), source, 0);
    walk_until(&mut e, |game| {
        matches!(
            game.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::UseShieldCounter { .. },
                ..
            }
        )
    });
    e.apply(them(), PlayerAction::YesNo(false)).unwrap();
    walk_until(&mut e, |game| {
        matches!(
            game.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayTax { .. },
                ..
            }
        )
    });
    for id in [source, stolen, ordinary] {
        assert_eq!(
            e.state().object(id).unwrap().zone,
            crate::zone::Zone::Graveyard
        );
    }
    assert_eq!(
        e.state().object(exiled).unwrap().zone,
        crate::zone::Zone::Exile
    );
    for id in [indestructible, regenerating] {
        assert_eq!(
            e.state().object(id).unwrap().zone,
            crate::zone::Zone::Battlefield
        );
    }
    let mut counts = [0, 0];
    let mut paid = false;
    while let Pending::YesNo {
        player,
        prompt: YesNoPrompt::PayTax { mana: 2 },
        ..
    } = e.pending().clone()
    {
        counts[usize::from(player.get())] += 1;
        let pay = player == me() && !paid;
        paid |= pay;
        e.apply(player, PlayerAction::YesNo(pay)).unwrap();
        if matches!(e.pending(), Pending::Priority { .. }) && !e.state().zones.stack_is_empty() {
            walk_until(&mut e, |game| {
                matches!(game.pending(), Pending::YesNo { .. })
                    || game.state().zones.stack_is_empty()
            });
        }
    }
    settle(&mut e, me());
    assert_eq!(
        counts,
        [3, 1],
        "source, stolen and exiled land count for their old controller; regeneration and indestructibility count zero"
    );
    assert_eq!(life(&e, me()), 18, "one of three payments was made");
    assert_eq!(life(&e, them()), 19);
}

#[test]
fn destroyed_this_way_counts_exile_replacement_but_not_failed_destruction_and_filters_departures() {
    use baylee_cards_dsl::ThisWay;
    static ABILITIES: &[AbilityDef] = &[free(
        &[
            Effect::DestroyAll {
                filter: &Filter::LAND,
                no_regen: false,
            },
            Effect::GainLife {
                amount: Amount::ThisWay(ThisWay::Destroyed(&PLAINS)),
            },
        ],
        None,
    )];
    let mut e = bench(
        8621,
        vec![
            card(
                SOURCE,
                land_face("Sweep source"),
                KeywordSet::EMPTY,
                ABILITIES,
            ),
            card(ORDINARY, plains("Ordinary plains"), KeywordSet::EMPTY, &[]),
            card(EXILED, plains("Exiling plains"), KeywordSet::EMPTY, EXILE),
            card(
                INDESTRUCTIBLE,
                plains("Indestructible plains"),
                KeywordSet::INDESTRUCTIBLE,
                &[],
            ),
            card(
                REGENERATING,
                plains("Regenerating plains"),
                KeywordSet::EMPTY,
                REGENERATE,
            ),
        ],
        [
            Seat::with(&[SOURCE, REGENERATING]),
            Seat::with(&[ORDINARY, EXILED, INDESTRUCTIBLE]),
        ],
    );
    to_main(&mut e, me());
    let regenerating = the(&e, ZoneLocation::Battlefield, REGENERATING);
    activate(&mut e, me(), regenerating, 0);
    settle(&mut e, me());
    let source = the(&e, ZoneLocation::Battlefield, SOURCE);
    activate(&mut e, me(), source, 0);
    settle(&mut e, me());
    assert_eq!(
        life(&e, me()),
        22,
        "only the ordinary and exiled Plains were destroyed; the source has no Plains subtype"
    );
    assert_eq!(
        e.state().object(regenerating).unwrap().zone,
        crate::zone::Zone::Battlefield
    );
    assert_eq!(
        objects(&e, ZoneLocation::Battlefield, INDESTRUCTIBLE).len(),
        1
    );
    assert_eq!(objects(&e, ZoneLocation::Exile(them()), EXILED).len(), 1);
}

#[test]
fn destroyed_this_way_excludes_sacrifice_and_direct_exile_in_the_same_resolution() {
    use baylee_cards_dsl::ThisWay;
    static ABILITIES: &[AbilityDef] = &[free(
        &[
            Effect::SacrificeAll { filter: &PLAINS },
            Effect::ExileAll {
                filter: &Filter::HasSubtype(land::FOREST),
                linked: false,
            },
            Effect::DestroyAll {
                filter: &Filter::LAND,
                no_regen: false,
            },
            Effect::GainLife {
                amount: Amount::ThisWay(ThisWay::Destroyed(&Filter::LAND)),
            },
        ],
        None,
    )];
    let mut e = bench(
        8622,
        vec![
            card(
                SOURCE,
                land_face("Sweep source"),
                KeywordSet::EMPTY,
                ABILITIES,
            ),
            card(
                ORDINARY,
                plains("Sacrificed plains"),
                KeywordSet::EMPTY,
                &[],
            ),
            card(
                EXILED,
                FaceDef {
                    subtypes: &[land::FOREST],
                    ..land_face("Exiled forest")
                },
                KeywordSet::EMPTY,
                &[],
            ),
        ],
        [Seat::with(&[SOURCE]), Seat::with(&[ORDINARY, EXILED])],
    );
    to_main(&mut e, me());
    let source = the(&e, ZoneLocation::Battlefield, SOURCE);
    activate(&mut e, me(), source, 0);
    settle(&mut e, me());
    assert_eq!(
        life(&e, me()),
        21,
        "only the source was destroyed; sacrifice and exile are different operations"
    );
    assert_eq!(
        objects(&e, ZoneLocation::Graveyard(them()), ORDINARY).len(),
        1
    );
    assert_eq!(objects(&e, ZoneLocation::Exile(them()), EXILED).len(), 1);
    let exiled = the(&e, ZoneLocation::Exile(them()), EXILED);
    assert!(
        !e.state()
            .object(exiled)
            .unwrap()
            .riders
            .iter()
            .any(|r| matches!(r, crate::object::Rider::Linked { .. })),
        "an ordinary exile sweep creates no linked-exile relationship"
    );
}
