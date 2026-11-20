//! `cards/instants/mv_1/trickery_charm.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
#[allow(clippy::wildcard_imports)]
use baylee_card_testkit::card_tests::legends_kit::*;
use baylee_core::generated::subtypes;

fn trickery_charm() -> CardIndex {
    card_index("bb97f7c6-6a75-4f4e-b74e-c57ce98a08fc")
}

/// Mode 0 grants target creature flying until end of turn, and wears off at cleanup.
#[test]
fn trickery_charm_mode_zero_grants_flying_until_eot() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), llanowar_elves()])
        .hand(0, &[trickery_charm()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("Elf seated");
    assert!(!keywords(&engine, elf).contains(KeywordSet::FLYING));

    tap_all_mana(&mut engine, p0);
    cast_modal(&mut engine, p0, trickery_charm(), 0, 0, &[elf], &[]);
    assert!(keywords(&engine, elf).contains(KeywordSet::FLYING));

    reach_their_main_phase(&mut engine, PlayerId::new(1));
    assert!(!keywords(&engine, elf).contains(KeywordSet::FLYING));
}

/// Mode 1 changes target creature to any chosen creature type until end of turn.
#[test]
fn trickery_charm_mode_one_changes_creature_type_until_eot() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), llanowar_elves()])
        .hand(0, &[trickery_charm()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("Elf seated");
    tap_all_mana(&mut engine, p0);
    announce(&mut engine, p0, trickery_charm(), 1, 0, &[elf], &[]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseSubtype { .. })
    });
    engine
        .apply(p0, PlayerAction::ChooseSubtype(subtypes::creature::DRAGON))
        .expect("choose Dragon for Trickery Charm mode 1");
    pass_until(&mut engine, |e| at_rest(e, p0));

    let subs = &engine
        .state()
        .object(elf)
        .unwrap()
        .characteristics()
        .subtypes;
    assert!(subs.contains(subtypes::creature::DRAGON));
    assert!(!subs.contains(subtypes::creature::ELF));

    reach_their_main_phase(&mut engine, PlayerId::new(1));
    let subs_after = &engine
        .state()
        .object(elf)
        .unwrap()
        .characteristics()
        .subtypes;
    assert!(subs_after.contains(subtypes::creature::ELF));
    assert!(!subs_after.contains(subtypes::creature::DRAGON));
}

/// Mode 2 lets you look at the top four cards of your library and put them back in any order.
#[test]
fn trickery_charm_mode_two_reorders_top_four_of_library() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island()])
        .hand(
            0,
            &[
                trickery_charm(),
                llanowar_elves(),
                festering_goblin(),
                serra_angel(),
                ornithopter(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let a = hand_to_library_top(&mut engine, p0, llanowar_elves());
    let b = hand_to_library_top(&mut engine, p0, festering_goblin());
    let c = hand_to_library_top(&mut engine, p0, serra_angel());
    let d = hand_to_library_top(&mut engine, p0, ornithopter());
    let lib_before = library_size(&engine, p0);

    tap_all_mana(&mut engine, p0);
    announce(&mut engine, p0, trickery_charm(), 2, 0, &[], &[]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Arrange { .. })
    });
    let Pending::Arrange { player, cards, .. } = engine.pending().clone() else {
        unreachable!("matched Arrange");
    };
    assert_eq!(player, p0);
    assert_eq!(cards.len(), 4);
    for id in [a, b, c, d] {
        assert!(cards.contains(&id), "top four cards include {id:?}");
    }

    // Reorder onto top of library with `a` topmost (`pile[0]` sits on top).
    engine
        .apply(
            p0,
            PlayerAction::Arrange {
                piles: vec![vec![a, b, c, d]],
            },
        )
        .expect("arrange top four");
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(library_size(&engine, p0), lib_before);
    let lib = engine.state().zones.list(ZoneLocation::Library(p0));
    let tail = &lib[lib.len() - 4..];
    assert_eq!(
        tail,
        [d, c, b, a],
        "top 4 cards lie on top of library with `a` on top"
    );
}
