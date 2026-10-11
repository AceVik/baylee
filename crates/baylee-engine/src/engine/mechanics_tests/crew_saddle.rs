//! Crew and saddle share exact payment weights without changing combat power.
use super::*;
use crate::object::{Rider, Status};
use baylee_cards_dsl::{Amount, CostPart, Filter, activated, cost, crew, saddle};
use baylee_core::generated::subtypes::{artifact, creature};

const MAKER: u32 = 7510;
const VEHICLE: u32 = 7511;
const MOUNT: u32 = 7512;
const ORDINARY: u32 = 7513;
static MAKE: &[AbilityDef] = &[
    free(
        &[Effect::CreateTokenN {
            token: &baylee_cards::tokens::PILOT_1_1_COLORLESS,
            amount: Amount::Fixed(1),
        }],
        None,
    ),
    free(
        &[Effect::CreateTokenN {
            token: &baylee_cards::tokens::PILOT_1_1_COLORLESS,
            amount: Amount::Fixed(64),
        }],
        None,
    ),
];
static CREW: &[AbilityDef] = &[
    crew!(3),
    crew!(0),
    activated!(cost!(Crew(3), Sacrifice(&Filter::CREATURE)), &[]),
];
static SADDLE: &[AbilityDef] = &[saddle!(3), saddle!(0)];

fn table() -> Bench {
    let mut vehicle = creature_face("Vehicle", "{0}", 2, 5);
    vehicle.types = TypeSet::ARTIFACT;
    vehicle.subtypes = &[artifact::VEHICLE];
    let mut mount = creature_face("Mount", "{0}", 2, 5);
    mount.subtypes = &[creature::MOUNT];
    let mut e = bench(
        7510,
        vec![
            super::super::synthetic::land(MAKER, "Maker", MAKE),
            card(VEHICLE, vehicle, KeywordSet::EMPTY, CREW),
            card(MOUNT, mount, KeywordSet::EMPTY, SADDLE),
            card(
                ORDINARY,
                creature_face("Ordinary", "{0}", 1, 5),
                KeywordSet::EMPTY,
                &[],
            ),
        ],
        [
            Seat::with(&[MAKER, VEHICLE, MOUNT, ORDINARY]),
            Seat::default(),
        ],
    );
    to_main(&mut e, me());
    e
}
fn pilot(e: &mut Bench) -> ObjectId {
    let maker = the(e, ZoneLocation::Battlefield, MAKER);
    activate(e, me(), maker, 0);
    settle(e, me());
    *e.state()
        .zones
        .list(ZoneLocation::Battlefield)
        .last()
        .unwrap()
}

#[test]
fn a_new_pilot_pays_three_for_crew_or_saddle_without_haste_or_extra_combat_power() {
    for saddle in [false, true] {
        let mut e = table();
        let pilot = pilot(&mut e);
        let source = the(
            &e,
            ZoneLocation::Battlefield,
            if saddle { MOUNT } else { VEHICLE },
        );
        activate(&mut e, me(), source, 0);
        let Pending::ChooseCards {
            options,
            total: Some(total),
            prompt,
            ..
        } = e.pending()
        else {
            panic!("power choice")
        };
        let pos = options.iter().position(|id| *id == pilot).unwrap();
        assert_eq!(total.weights[pos], 3);
        assert!(matches!(
            (saddle, prompt),
            (false, crate::choice::ChoicePrompt::CostCrew { power: 3 })
                | (true, crate::choice::ChoicePrompt::CostSaddle { power: 3 })
        ));
        let ordinary = the(&e, ZoneLocation::Battlefield, ORDINARY);
        let before = e.state().snapshot_hash();
        assert!(
            e.apply(
                me(),
                PlayerAction::ChooseObjects {
                    objects: vec![ordinary]
                }
            )
            .is_err()
        );
        assert_eq!(e.state().snapshot_hash(), before);
        e.apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![pilot],
            },
        )
        .unwrap();
        assert!(
            e.state()
                .object(pilot)
                .unwrap()
                .status
                .contains(Status::TAPPED)
        );
        assert!(
            !e.state()
                .object(source)
                .unwrap()
                .riders
                .contains(&Rider::Saddled)
        );
        settle(&mut e, me());
        assert_eq!(
            e.state().object(pilot).unwrap().characteristics().power,
            Some(1)
        );
        assert_eq!(
            e.state()
                .object(source)
                .unwrap()
                .riders
                .contains(&Rider::Saddled),
            saddle
        );
        if !saddle {
            assert!(
                e.state()
                    .object(source)
                    .unwrap()
                    .characteristics()
                    .types
                    .contains(TypeSet::CREATURE)
            );
        }
        to_main(&mut e, them());
        assert!(
            !e.state()
                .object(source)
                .unwrap()
                .riders
                .contains(&Rider::Saddled)
        );
    }
}

#[test]
fn zero_crew_and_saddle_accept_no_creatures() {
    for index in [VEHICLE, MOUNT] {
        let mut e = table();
        let source = the(&e, ZoneLocation::Battlefield, index);
        activate(&mut e, me(), source, 1);
        e.apply(me(), PlayerAction::ChooseObjects { objects: vec![] })
            .unwrap();
        settle(&mut e, me());
        let source = e.state().object(source).unwrap();
        assert!(source.characteristics().types.contains(TypeSet::CREATURE));
        assert_eq!(source.riders.contains(&Rider::Saddled), index == MOUNT);
    }
}

#[test]
fn pilot_bonus_uses_current_source_types_and_does_not_apply_to_every_power_payment() {
    let mut e = table();
    let pilot = pilot(&mut e);
    let vehicle = the(&e, ZoneLocation::Battlefield, VEHICLE);
    let ordinary = the(&e, ZoneLocation::Battlefield, ORDINARY);
    let plan = |source, part| {
        super::super::cost_wizard::PowerPayment::new(e.state(), source, part).weight(pilot)
    };
    assert_eq!(plan(vehicle, &CostPart::Crew(3)), 3);
    assert_eq!(plan(vehicle, &CostPart::Saddle(3)), 1);
    assert_eq!(plan(ordinary, &CostPart::Crew(3)), 1);
    assert_eq!(plan(ordinary, &CostPart::Saddle(3)), 1);
}

#[test]
fn saddle_is_sorcery_speed_and_cannot_mark_a_new_source_incarnation() {
    let mut e = table();
    let pilot = pilot(&mut e);
    let source = the(&e, ZoneLocation::Battlefield, MOUNT);
    activate(&mut e, me(), source, 0);
    e.apply(
        me(),
        PlayerAction::ChooseObjects {
            objects: vec![pilot],
        },
    )
    .unwrap();
    let state = e.dev_state_mut(me()).unwrap();
    state
        .move_object(
            source,
            ZoneLocation::Exile(me()),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    state
        .move_object(
            source,
            ZoneLocation::Battlefield,
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    settle(&mut e, me());
    assert!(
        !e.state()
            .object(source)
            .unwrap()
            .riders
            .contains(&Rider::Saddled)
    );
    to_main(&mut e, them());
    e.apply(them(), PlayerAction::PassPriority).unwrap();
    assert!(
        e.apply(
            me(),
            PlayerAction::ActivateAbility {
                source,
                ability_index: 1
            }
        )
        .is_err()
    );
}

#[test]
fn indexed_power_weights_match_a_full_effect_scan_with_negative_power_and_overlapping_bonuses() {
    use crate::effects::{ContinuousEffect, EffectFilter, EffectOrigin};
    use baylee_cards_dsl::{Duration, Layer, Modifier};
    use baylee_core::ids::EffectId;
    let mut e = table();
    let maker = the(&e, ZoneLocation::Battlefield, MAKER);
    activate(&mut e, me(), maker, 1);
    settle(&mut e, me());
    let ids: Vec<_> = e
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| e.state().object(*id).unwrap().token.is_some())
        .collect();
    let state = e.dev_state_mut(me()).unwrap();
    for (i, &id) in ids.iter().enumerate() {
        state.object_mut(id).unwrap().base_mut().power = Some(i16::try_from(i % 13).unwrap() - 6);
        let timestamp = state.next_timestamp();
        state.effects.register(ContinuousEffect {
            id: EffectId::new(0),
            source: Some(maker),
            controller: me(),
            origin: EffectOrigin::Resolution,
            layer: Layer::Text,
            timestamp,
            duration: Duration::UntilEndOfTurn,
            filter: EffectFilter::object(state, id),
            modifier: Modifier::CrewSaddlePowerBonus(i16::try_from(i % 7).unwrap() - 3),
        });
    }
    state.refresh_characteristics();
    let source = the(&e, ZoneLocation::Battlefield, VEHICLE);
    let payment =
        super::super::cost_wizard::PowerPayment::new(e.state(), source, &CostPart::Crew(3));
    let mut expected = Vec::new();
    for &id in &ids {
        let object = e.state().object(id).unwrap();
        let reference = e.state().effects.iter().fold(
            i32::from(object.characteristics().power.unwrap()),
            |n, effect| {
                if let Modifier::CrewSaddlePowerBonus(bonus) = effect.modifier
                    && crate::effects::applies_to(e.state(), effect, object)
                {
                    n + i32::from(bonus)
                } else {
                    n
                }
            },
        );
        assert_eq!(payment.weight(id), reference);
        expected.push(reference);
    }
    let positive: i64 = expected.iter().map(|n| i64::from((*n).max(0))).sum();
    for threshold in 0..=u8::MAX {
        assert_eq!(
            payment.can_reach(&ids, threshold),
            positive >= i64::from(threshold)
        );
    }
}

#[test]
fn a_multi_creature_answer_does_not_skip_the_next_cost_question() {
    let mut e = table();
    let pilot = pilot(&mut e);
    let source = the(&e, ZoneLocation::Battlefield, VEHICLE);
    let mount = the(&e, ZoneLocation::Battlefield, MOUNT);
    let ordinary = the(&e, ZoneLocation::Battlefield, ORDINARY);
    activate(&mut e, me(), source, 2);
    e.apply(
        me(),
        PlayerAction::ChooseObjects {
            objects: vec![mount, ordinary],
        },
    )
    .unwrap();
    assert!(matches!(
        e.pending(),
        Pending::ChooseCards {
            prompt: crate::choice::ChoicePrompt::CostSacrifice,
            ..
        }
    ));
    // No cost is paid until both answers have arrived.
    assert!(
        !e.state()
            .object(mount)
            .unwrap()
            .status
            .contains(Status::TAPPED)
    );
    e.apply(
        me(),
        PlayerAction::ChooseObjects {
            objects: vec![pilot],
        },
    )
    .unwrap();
    settle(&mut e, me());
    assert!(
        e.state()
            .object(mount)
            .unwrap()
            .status
            .contains(Status::TAPPED)
    );
    assert!(
        e.state()
            .object(ordinary)
            .unwrap()
            .status
            .contains(Status::TAPPED)
    );
    assert!(
        !e.state()
            .zones
            .list(ZoneLocation::Battlefield)
            .contains(&pilot)
    );
}

#[test]
fn removing_a_pilots_abilities_removes_its_payment_bonus() {
    use crate::effects::{ContinuousEffect, EffectFilter, EffectOrigin};
    use baylee_cards_dsl::{Duration, Layer, Modifier};
    use baylee_core::ids::EffectId;
    let mut e = table();
    let pilot = pilot(&mut e);
    let source = the(&e, ZoneLocation::Battlefield, VEHICLE);
    let state = e.dev_state_mut(me()).unwrap();
    let timestamp = state.next_timestamp();
    state.effects.register(ContinuousEffect {
        id: EffectId::new(0),
        source: Some(source),
        controller: me(),
        origin: EffectOrigin::Resolution,
        layer: Layer::Ability,
        timestamp,
        duration: Duration::UntilEndOfTurn,
        filter: EffectFilter::object(state, pilot),
        modifier: Modifier::LoseAllAbilities,
    });
    state.refresh_characteristics();
    e.apply(me(), PlayerAction::PassPriority).unwrap();
    assert_eq!(
        super::super::cost_wizard::PowerPayment::new(e.state(), source, &CostPart::Crew(3))
            .weight(pilot),
        1
    );
}
