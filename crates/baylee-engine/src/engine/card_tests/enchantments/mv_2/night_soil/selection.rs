//! Every offered first card must select a payable graveyard, never a hidden group constraint.
#[allow(clippy::wildcard_imports)]
use super::*;
use crate::object::AbilityList;
use baylee_cards_dsl::{AbilityDef, Amount, Cost, CostPart, Effect, Filter, ManaCost};

fn payment(count: u8, other_count: u8) -> Engine<RegistryLookup> {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[
                soil_card(),
                forest(),
                bears_card(),
                hill_giant_card(),
                bears_card(),
            ],
        )
        .battlefield(1, &[bears_card(), hill_giant_card(), bears_card()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    for (seat, available) in [(P0, count), (P1, other_count)] {
        let creatures: Vec<_> = engine
            .state()
            .battlefield_seen()
            .filter(|id| {
                engine.state().object(*id).is_some_and(|object| {
                    object.controller == seat
                        && object.characteristics().types.contains(TypeSet::CREATURE)
                })
            })
            .take(usize::from(available))
            .collect();
        assert_eq!(creatures.len(), usize::from(available));
        bury(&mut engine, &creatures);
    }
    if count == 3 {
        // A synthetic counted cost exercises the existing continuation beyond Night Soil's two.
        static ABILITIES: &[AbilityDef] = &[baylee_cards_dsl::activated!(
            Cost {
                mana: ManaCost::ZERO,
                parts: &[CostPart::ExileFromSingleGraveyard {
                    filter: &Filter::CREATURE,
                    count: 3,
                }]
            },
            &[Effect::GainLife {
                amount: Amount::Fixed(1)
            }]
        )];
        let source = on_battlefield(&engine, P0, soil_card()).unwrap();
        engine
            .state
            .object_mut(source)
            .unwrap()
            .take_abilities(AbilityList::from_static(ABILITIES, None, None));
    }
    engine.refresh_offer();
    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, soil_card(), 0);
    engine
}

fn menu(engine: &Engine<RegistryLookup>, count: u8) -> Vec<ObjectId> {
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending()
    else {
        panic!("expected exile cost, got {:?}", engine.pending());
    };
    assert_eq!((*player, *min, *max), (P0, count, count));
    options.clone()
}

fn choose(engine: &mut Engine<RegistryLookup>, objects: Vec<ObjectId>) {
    let answer = PlayerAction::ChooseObjects { objects };
    assert!(engine.pending().answer_fault(&answer).is_none());
    engine
        .apply(P0, answer)
        .expect("every offered answer must be accepted");
}

#[test]
fn every_first_card_selects_one_graveyard_and_costs_remain_atomic() {
    for count in [2, 3] {
        for first in 0..usize::from(count) * 2 {
            let mut engine = payment(count, count);
            let options = menu(&engine, 1);
            assert_eq!(options.len(), usize::from(count) * 2);
            let chosen = options[first];
            let owner = engine.state().object(chosen).unwrap().zone_owner.unwrap();
            let other = options
                .iter()
                .copied()
                .find(|id| engine.state().object(*id).unwrap().zone_owner != Some(owner))
                .unwrap();
            let mana = engine.state().players[0].mana_pool.total();
            choose(&mut engine, vec![chosen]);
            let remaining = menu(&engine, count - 1);
            assert_eq!(remaining.len(), usize::from(count - 1));
            assert!(!remaining.contains(&chosen));
            assert!(
                remaining
                    .iter()
                    .all(|id| engine.state().object(*id).unwrap().zone_owner == Some(owner))
            );
            assert_eq!(engine.state().players[0].mana_pool.total(), mana);
            for seat in [P0, P1] {
                assert_eq!(
                    engine
                        .state()
                        .zones
                        .list(ZoneLocation::Graveyard(seat))
                        .len(),
                    usize::from(count)
                );
            }
            let before = engine.state().snapshot_hash();
            let question = format!("{:?}", engine.pending());
            let mut mixed = remaining.clone();
            mixed[0] = other;
            let bad = PlayerAction::ChooseObjects { objects: mixed };
            assert_eq!(
                engine.pending().answer_fault(&bad),
                Some(crate::choice::AnswerFault::NotOffered)
            );
            assert!(engine.apply(P0, bad).is_err());
            assert_eq!(engine.state().snapshot_hash(), before);
            assert_eq!(format!("{:?}", engine.pending()), question);
            choose(&mut engine, remaining);
            pass_until(&mut engine, stack_is_empty);
            assert!(
                engine
                    .state()
                    .zones
                    .list(ZoneLocation::Graveyard(owner))
                    .is_empty()
            );
            assert_eq!(
                engine.state().zones.list(ZoneLocation::Exile(owner)).len(),
                usize::from(count)
            );
            let other_owner = if owner == P0 { P1 } else { P0 };
            assert_eq!(
                engine
                    .state()
                    .zones
                    .list(ZoneLocation::Graveyard(other_owner))
                    .len(),
                usize::from(count)
            );
            if count == 2 {
                assert_eq!(saprolings(&engine).len(), 1);
            }
        }
    }
}

#[test]
fn only_one_payable_graveyard_keeps_the_original_batch() {
    for count in [2, 3] {
        let mut engine = payment(count, count - 1);
        let options = menu(&engine, count);
        assert_eq!(options.len(), usize::from(count));
        assert!(
            options
                .iter()
                .all(|id| engine.state().object(*id).unwrap().zone_owner == Some(P0))
        );
        choose(&mut engine, options);
        pass_until(&mut engine, stack_is_empty);
        assert!(
            engine
                .state()
                .zones
                .list(ZoneLocation::Graveyard(P0))
                .is_empty()
        );
        assert_eq!(
            engine.state().zones.list(ZoneLocation::Graveyard(P1)).len(),
            usize::from(count - 1)
        );
    }
}
