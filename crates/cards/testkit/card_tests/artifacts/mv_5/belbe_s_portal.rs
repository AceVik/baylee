//! `cards/artifacts/mv_5/belbe_s_portal.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::subtypes;

fn belbe_s_portal() -> CardIndex {
    card_index("46e50ee9-91f8-416e-82fa-f1a76e61fb73")
}

fn woodland_changeling() -> CardIndex {
    card_index("338f0ab6-f744-4e5a-a358-05e71b93758d")
}

/// Belbe's Portal names a creature type as it enters and only lets its
/// controller put a creature card of that type (including changeling
/// creatures, excluding wrong-type creatures and noncreature changeling
/// spells) from hand onto the battlefield, optionally.
#[test]
fn belbe_s_portal_puts_matching_creature_from_hand_and_may_decline() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .hand(
            0,
            &[
                belbe_s_portal(),
                llanowar_elves(),
                woodland_changeling(),
                festering_goblin(),
                crib_swap(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, belbe_s_portal());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseSubtype { .. })
    });
    engine
        .apply(p0, PlayerAction::ChooseSubtype(subtypes::creature::ELF))
        .expect("choose Elf as Belbe's Portal enters");
    pass_until(&mut engine, stack_is_empty);

    let portal = on_battlefield(&engine, p0, belbe_s_portal()).expect("Belbe's Portal entered");
    let elf_in_hand = in_hand(&engine, p0, llanowar_elves()).expect("Elf in hand");
    let changeling_in_hand =
        in_hand(&engine, p0, woodland_changeling()).expect("Changeling in hand");
    let goblin_in_hand = in_hand(&engine, p0, festering_goblin()).expect("Goblin in hand");
    let noncreature_changeling = in_hand(&engine, p0, crib_swap()).expect("Crib Swap in hand");

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, belbe_s_portal(), 0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("matched ChooseCards");
    };
    assert_eq!(player, p0);
    assert_eq!(
        min, 0,
        "putting a creature onto the battlefield is optional"
    );
    assert_eq!(max, 1);
    assert!(
        options.contains(&elf_in_hand),
        "Elf creature card is offered"
    );
    assert!(
        options.contains(&changeling_in_hand),
        "Changeling creature card counts as the chosen type"
    );
    assert!(
        !options.contains(&goblin_in_hand),
        "Goblin of another type is excluded"
    );
    assert!(
        !options.contains(&noncreature_changeling),
        "noncreature Kindred card is not a creature card"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elf_in_hand],
            },
        )
        .expect("put Elf onto battlefield");
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(
        is_tapped(&engine, portal),
        "activated ability tapped portal"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "chosen Elf entered the battlefield"
    );
    assert!(
        in_hand(&engine, p0, llanowar_elves()).is_none(),
        "Elf left hand"
    );

    // Next turn, portal untaps: activate again and decline (`vec![]`).
    reach_their_main_phase(&mut engine, PlayerId::new(1));
    reach_their_main_phase(&mut engine, p0);
    let pool_before = lands_of(&engine, p0).len();
    assert!(pool_before >= 3);
    tap_mana_where(&mut engine, p0, |_| true);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: portal,
                ability_index: 0,
            },
        )
        .expect("activate Belbe's Portal second time");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![] })
        .expect("decline putting Woodland Changeling");
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(
        in_hand(&engine, p0, woodland_changeling()).is_some(),
        "declining leaves Changeling in hand"
    );
    assert!(on_battlefield(&engine, p0, woodland_changeling()).is_none());
}
