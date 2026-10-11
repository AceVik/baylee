//! `cards/lands/deserts/lazotep_quarry.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

#[test]
fn lazotep_quarry_sacrifices_creature_to_produce_colored_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(307, forest())
        .battlefield(0, &[lazotep_quarry(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("elf deployed");
    activate(&mut engine, p0, lazotep_quarry(), 1);

    let Pending::ChooseCards {
        options, prompt, ..
    } = engine.pending().clone()
    else {
        panic!("expected sacrifice choice, got {:?}", engine.pending());
    };
    assert_eq!(prompt, crate::choice::ChoicePrompt::CostSacrifice);
    assert!(options.contains(&elf));

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .unwrap();

    let Pending::ChooseColor { .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .unwrap();

    assert!(in_graveyard(&engine, p0, llanowar_elves()).is_some());
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Black), 1);
    let quarry = on_battlefield(&engine, p0, lazotep_quarry()).expect("quarry on battlefield");
    assert!(is_tapped(&engine, quarry));
}

#[test]
fn quarry_exiles_the_announced_value_and_copies_abilities_with_zombie_exceptions() {
    use baylee_core::generated::{index, subtypes};
    let p0 = PlayerId::new(0);
    for doubled in [false, true] {
        for gone in [false, true] {
            let mut board = vec![lazotep_quarry(), llanowar_elves(), index::GRIZZLY_BEARS];
            if doubled {
                board.push(index::DOUBLING_SEASON);
            }
            let mut e = Duel::new(SEED, forest()).battlefield(0, &board).start();
            keep_mulligans(&mut e);
            reach_main_phase(&mut e, p0);
            let land = on_battlefield(&e, p0, lazotep_quarry()).unwrap();
            let elf = on_battlefield(&e, p0, llanowar_elves()).unwrap();
            let bear = on_battlefield(&e, p0, index::GRIZZLY_BEARS).unwrap();
            bury(&mut e, &[elf, bear]);
            e.dev_state_mut(p0).unwrap().players[0]
                .mana_pool
                .add(ManaColor::Colorless, 3);
            e.refresh_offer();
            activate(&mut e, p0, lazotep_quarry(), 2);
            assert!(matches!(e.pending(), Pending::ChooseNumber { .. }));
            e.apply(p0, PlayerAction::ChooseNumber(1)).unwrap();
            let Pending::ChooseTargets { options, .. } = e.pending() else {
                panic!("targets: {:?}", e.pending())
            };
            assert!(options.contains(&elf));
            assert!(!options.contains(&bear));
            aim_at(&mut e, p0, elf);
            assert!(matches!(
                e.pending(),
                Pending::ChooseCards {
                    prompt: ChoicePrompt::CostSacrifice,
                    ..
                }
            ));
            e.apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![land],
                },
            )
            .unwrap();
            if gone {
                e.dev_state_mut(p0)
                    .unwrap()
                    .move_object(
                        elf,
                        ZoneLocation::Exile(p0),
                        ZonePosition::Top,
                        Cause::Effect,
                    )
                    .unwrap();
            }
            pass_until(&mut e, stack_is_empty);
            assert_eq!(e.state().object(land).unwrap().zone, Zone::Graveyard);
            assert_eq!(e.state().object(elf).unwrap().zone, Zone::Exile);
            let copies: Vec<_> = e
                .state()
                .zones
                .list(ZoneLocation::Battlefield)
                .iter()
                .copied()
                .filter(|id| e.state().object(*id).unwrap().card.is_none())
                .collect();
            assert_eq!(
                copies.len(),
                if gone {
                    0
                } else if doubled {
                    2
                } else {
                    1
                }
            );
            for id in copies {
                let object = e.state().object(id).unwrap();
                let chars = object.characteristics();
                assert_eq!(pt(&e, id), (4, 4));
                assert_eq!(
                    chars.colors,
                    baylee_core::color::ColorSet::of(baylee_core::color::Color::Black)
                );
                assert!(chars.subtypes.contains(subtypes::creature::ZOMBIE));
                assert!(!chars.subtypes.contains(subtypes::creature::ELF));
                assert!(!chars.subtypes.contains(subtypes::creature::DRUID));
                assert!(
                    e.state()
                        .printed_ability_list(id)
                        .unwrap()
                        .abilities
                        .iter()
                        .any(baylee_cards_dsl::AbilityDef::is_mana_ability)
                );
            }
        }
    }
}
