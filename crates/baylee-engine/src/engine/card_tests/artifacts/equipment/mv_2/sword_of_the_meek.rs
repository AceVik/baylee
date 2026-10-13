//! `cards/artifacts/equipment/mv_2/sword_of_the_meek.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

#[test]
fn sword_of_the_meek_arms_the_creature_it_targets_and_no_other() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                llanowar_elves(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[sword_of_the_meek()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one of which stays bare");
    let (host, bystander) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert_eq!(pt(&engine, host), (1, 1), "a printed 1/1 before the Sword");

    // {2} off two of the four Forests; the other two pay the equip.
    cast_from_hand(&mut engine, p0, sword_of_the_meek());
    pass_until(&mut engine, stack_is_empty);
    let sword = on_battlefield(&engine, p0, sword_of_the_meek()).expect("the Sword resolved");
    assert!(
        engine
            .state()
            .object(sword)
            .is_some_and(|o| o.attached_to.is_none()),
        "with nothing chosen yet it enters holding nobody"
    );
    assert_eq!(
        pt(&engine, host),
        (1, 1),
        "an Equipment attached to nothing modifies nothing"
    );

    // Ability 1 is Equip {2}; ability 0 is the static that grants.
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, sword_of_the_meek(), 1);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "equip targets a creature you control, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&host) && options.contains(&bystander),
        "both creatures you control may be armed: {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "\"target creature *you* control\" declines the Elf across the table: {options:?}"
    );
    assert!(
        !options.contains(&sword),
        "the Equipment is an artifact and no creature: {options:?}"
    );
    assert_eq!(options.len(), 2, "and those two are the whole menu");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state()
            .object(sword)
            .is_some_and(|o| o.attached_to == Some(host))
    });

    assert_eq!(
        pt(&engine, host),
        (2, 3),
        "+1/+2 on the creature the Sword is attached to"
    );
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "the Elf nobody equipped is still the 1/1 it was printed as"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "the static reaches the equipped creature and never across the table"
    );
}

#[test]
fn sword_returns_untargeted_but_never_attaches_to_a_new_incarnation_or_returns_a_new_sword() {
    use baylee_core::generated::index;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for case in 0..6 {
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &[index::SWORD_OF_THE_MEEK])
            .hand(0, &[index::LLANOWAR_ELVES, index::GRIZZLY_BEARS])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let sword = on_battlefield(&e, p0, index::SWORD_OF_THE_MEEK).unwrap();
        bury(&mut e, &[sword]);
        e.dev_state_mut(p0).unwrap().players[0]
            .mana_pool
            .add(ManaColor::Green, 3);
        e.refresh_offer();
        let creature = if case == 5 {
            index::GRIZZLY_BEARS
        } else {
            index::LLANOWAR_ELVES
        };
        cast_with_floating(&mut e, p0, creature);
        pass_until(&mut e, |e| on_battlefield(e, p0, creature).is_some());
        let elf = on_battlefield(&e, p0, creature).unwrap();
        if case == 5 {
            assert!(stack_is_empty(&e));
            continue;
        }
        if case == 1 {
            e.dev_state_mut(p0)
                .unwrap()
                .object_mut(elf)
                .unwrap()
                .set_controller(p1);
        }
        if case == 2 {
            leave_and_return(&mut e, elf, ZoneLocation::Battlefield);
            // The new creature creates another trigger; decline it so only the old one is tested.
            pass_until(&mut e, |e| {
                matches!(
                    e.pending(),
                    Pending::YesNo {
                        prompt: YesNoPrompt::MayDo,
                        ..
                    }
                )
            });
            e.apply(p0, PlayerAction::YesNo(false)).unwrap();
        }
        if case == 3 {
            leave_and_return(&mut e, sword, ZoneLocation::Graveyard(p0));
        }
        if case == 4 {
            e.dev_state_mut(p0)
                .unwrap()
                .object_mut(elf)
                .unwrap()
                .base_mut()
                .keywords = baylee_cards_dsl::KeywordSet::SHROUD;
        }
        pass_until(&mut e, stack_is_empty);
        assert_eq!(
            e.state().object(sword).unwrap().zone,
            if case == 3 {
                Zone::Graveyard
            } else {
                Zone::Battlefield
            },
            "case {case}"
        );
        assert_eq!(
            e.state().object(sword).unwrap().attached_to,
            if matches!(case, 2 | 3) {
                None
            } else {
                Some(elf)
            },
            "case {case}"
        );
    }
}

#[test]
fn sword_observes_entries_while_its_own_incarnation_was_in_the_graveyard() {
    use baylee_core::generated::index;
    let p0 = PlayerId::new(0);
    for there_during_entry in [false, true] {
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &[index::SWORD_OF_THE_MEEK])
            .hand(0, &[index::LLANOWAR_ELVES])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let sword = on_battlefield(&e, p0, index::SWORD_OF_THE_MEEK).unwrap();
        let elf = in_hand(&e, p0, index::LLANOWAR_ELVES).unwrap();
        if there_during_entry {
            bury(&mut e, &[sword]);
            e.refresh_offer();
        }
        let state = e.dev_state_mut(p0).unwrap();
        state.object_mut(elf).unwrap().kind = crate::object::ObjectKind::Permanent;
        state
            .move_object(
                elf,
                ZoneLocation::Battlefield,
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .unwrap();
        if there_during_entry {
            state
                .move_object(
                    sword,
                    ZoneLocation::Exile(p0),
                    crate::zone::ZonePosition::Top,
                    crate::event::Cause::Effect,
                )
                .unwrap();
        } else {
            state
                .move_object(
                    sword,
                    ZoneLocation::Graveyard(p0),
                    crate::zone::ZonePosition::Top,
                    crate::event::Cause::Effect,
                )
                .unwrap();
        }
        e.apply(p0, PlayerAction::PassPriority).unwrap();
        assert_eq!(
            e.state().zones.list(ZoneLocation::Stack).len(),
            usize::from(there_during_entry)
        );
        pass_until(&mut e, stack_is_empty);
        assert_eq!(
            e.state().object(sword).unwrap().zone,
            if there_during_entry {
                Zone::Exile
            } else {
                Zone::Graveyard
            }
        );
    }
}

/// "Whenever a 1/1 creature **you control** enters": the same entry under the
/// opponent's control puts nothing on the stack and the Sword stays buried.
/// The p0 pass is the control: it is the one the Sword answers.
#[test]
fn sword_ignores_a_one_one_that_enters_under_an_opponents_control() {
    use baylee_core::generated::index;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for entering_under in [p0, p1] {
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &[index::SWORD_OF_THE_MEEK])
            .hand(0, &[index::LLANOWAR_ELVES])
            .hand(1, &[index::LLANOWAR_ELVES])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let sword = on_battlefield(&e, p0, index::SWORD_OF_THE_MEEK).unwrap();
        let elf = in_hand(&e, entering_under, index::LLANOWAR_ELVES).unwrap();
        bury(&mut e, &[sword]);
        e.refresh_offer();
        let state = e.dev_state_mut(p0).unwrap();
        state.object_mut(elf).unwrap().kind = crate::object::ObjectKind::Permanent;
        state
            .move_object(
                elf,
                ZoneLocation::Battlefield,
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .unwrap();
        assert_eq!(
            e.state().object(elf).unwrap().controller,
            entering_under,
            "the 1/1 entered under {entering_under:?}"
        );
        assert_eq!(pt(&e, elf), (1, 1), "and it is a 1/1");
        e.apply(p0, PlayerAction::PassPriority).unwrap();
        assert_eq!(
            e.state().zones.list(ZoneLocation::Stack).len(),
            usize::from(entering_under == p0),
            "the Sword's trigger is put on the stack for a 1/1 of its controller's, and for no other"
        );
        pass_until(&mut e, stack_is_empty);
        assert_eq!(
            e.state().object(sword).unwrap().zone,
            if entering_under == p0 {
                Zone::Battlefield
            } else {
                Zone::Graveyard
            },
            "the Sword returns only for its own controller's 1/1"
        );
    }
}

fn leave_and_return(e: &mut Engine<RegistryLookup>, id: ObjectId, destination: ZoneLocation) {
    let p0 = PlayerId::new(0);
    let state = e.dev_state_mut(p0).unwrap();
    state
        .move_object(
            id,
            ZoneLocation::Exile(p0),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    if destination == ZoneLocation::Battlefield {
        state.object_mut(id).unwrap().kind = crate::object::ObjectKind::Permanent;
    }
    state
        .move_object(
            id,
            destination,
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
}
