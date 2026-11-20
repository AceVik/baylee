//! `cards/artifacts/mv_6/doom_cannon.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
#[allow(clippy::wildcard_imports)]
use baylee_card_testkit::card_tests::legends_kit::*;
use baylee_core::generated::subtypes;

fn doom_cannon() -> CardIndex {
    card_index("c761af28-d7d8-46d3-9bc7-1e44fdf6d0a0")
}

fn woodland_changeling() -> CardIndex {
    card_index("338f0ab6-f744-4e5a-a358-05e71b93758d")
}

/// Doom Cannon chooses a creature type as it enters, requires sacrificing a
/// creature of that type as part of `{3}, {T}` activation, and deals 3 damage
/// to any target (creature or player).
#[test]
fn doom_cannon_sacrifices_chosen_type_to_damage_player_or_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
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
                forest(),
                llanowar_elves(),
                woodland_changeling(),
                festering_goblin(),
            ],
        )
        .battlefield(1, &[serra_angel()])
        .hand(0, &[doom_cannon()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, doom_cannon());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseSubtype { .. })
    });
    engine
        .apply(p0, PlayerAction::ChooseSubtype(subtypes::creature::ELF))
        .expect("choose Elf for Doom Cannon");
    pass_until(&mut engine, stack_is_empty);

    let cannon = on_battlefield(&engine, p0, doom_cannon()).expect("Doom Cannon entered");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("Elf seated");
    let changeling = on_battlefield(&engine, p0, woodland_changeling()).expect("Changeling seated");
    let goblin = on_battlefield(&engine, p0, festering_goblin()).expect("Goblin seated");
    let angel = on_battlefield(&engine, p1, serra_angel()).expect("opponent Angel seated");

    // Tap 3 lands, activate Doom Cannon targeting P1; only Elf & Changeling are
    // legal sacrifice payments.
    let tap_ids: Vec<ObjectId> = lands_of(&engine, p0).into_iter().take(3).collect();
    tap_mana_where(&mut engine, p0, |id| tap_ids.contains(&id));
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: cannon,
                ability_index: 0,
            },
        )
        .expect("start activating Doom Cannon");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    aim(&mut engine, &[], &[p1]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        unreachable!("matched sacrifice choice");
    };
    assert!(options.contains(&elf), "Elf may be sacrificed");
    assert!(options.contains(&changeling), "Changeling is also an Elf");
    assert!(
        !options.contains(&goblin),
        "non-Elf Goblin cannot pay the Elf sacrifice cost"
    );

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("sacrifice Llanowar Elves");
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(
        engine.state().players[1].life,
        17,
        "Doom Cannon dealt 3 damage to opponent"
    );
    assert_eq!(
        engine.state().object(elf).unwrap().zone,
        Zone::Graveyard,
        "sacrificed Elf is in the graveyard"
    );
    assert!(is_tapped(&engine, cannon));

    // Next turn, sacrifice Changeling to deal 3 damage to opponent's 4/4 Angel.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    let tap_ids: Vec<ObjectId> = lands_of(&engine, p0).into_iter().take(3).collect();
    tap_mana_where(&mut engine, p0, |id| tap_ids.contains(&id));
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: cannon,
                ability_index: 0,
            },
        )
        .expect("activate Doom Cannon on creature target");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    aim(&mut engine, &[angel], &[]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![changeling],
            },
        )
        .expect("sacrifice Changeling");
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(
        engine.state().object(angel).unwrap().damage,
        3,
        "Serra Angel has 3 damage marked"
    );
}

/// Without a creature of the chosen type to sacrifice, Doom Cannon's ability is not offered.
#[test]
fn doom_cannon_cannot_activate_without_creature_of_chosen_type() {
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
                forest(),
                festering_goblin(),
            ],
        )
        .hand(0, &[doom_cannon()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, doom_cannon());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseSubtype { .. })
    });
    engine
        .apply(p0, PlayerAction::ChooseSubtype(subtypes::creature::DRAGON))
        .expect("choose Dragon");
    pass_until(&mut engine, |e| at_rest(e, p0));

    let cannon = on_battlefield(&engine, p0, doom_cannon()).expect("Doom Cannon entered");
    tap_all_mana(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected priority");
    };
    assert!(
        !legal
            .abilities
            .iter()
            .any(|(id, idx)| *id == cannon && *idx == 0),
        "ability requires sacrificing a Dragon, and none is controlled"
    );
}
