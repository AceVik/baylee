//! `cards/enchantments/mv_2/circle_of_protection_green.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Circle of Protection: Green, played the same way against a green
/// attacker (Llanowar Elves).
#[test]
fn circle_of_protection_green_prevents_damage_from_a_chosen_green_attacker() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let cop = circle_of_protection_green();
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[cop, forest()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves");
    reach_their_main_phase(&mut engine, p1);

    let blockers = attack_and_collect_blocks(&mut engine, elf, p0);
    assert!(blockers.is_empty(), "p0 has nothing to block with");
    engine
        .apply(p0, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, cop, 0);
    let selected = baylee_core::ids::DamageSourceRef {
        object: elf,
        version: engine.state().object(elf).expect("source exists").version,
    };
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseDamageSource { .. })
    });
    let Pending::ChooseDamageSource {
        player,
        options,
        choice,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("the pass waited for exactly this")
    };
    assert_eq!(player, p0);
    assert_eq!(
        options,
        vec![selected],
        "the only green source on the board"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseDamageSource {
                choice,
                source: selected,
            },
        )
        .expect("off the list");

    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
    });
    assert_eq!(
        engine.state().players[0].life,
        20,
        "the Elves' combat damage was prevented"
    );
}

/// Circle of Protection: Green: "The next time a green source of your choice would deal damage to
/// you this turn, prevent that damage." Three attackers: the 3/3 Elephant chosen is prevented; the Elf deals 1 and the red Ogre 2.
/// The offer holds both green sources and not the Ogre, and the shield is
/// spent by the one hit it was for.
#[test]
fn circle_of_protection_green_prevents_one_chosen_source_once_and_only_green_ones_are_offered() {
    a_circle_prevents_the_next_damage_of_one_chosen_source(
        circle_of_protection_green(),
        wild_elephant(),
        llanowar_elves(),
        gray_ogre(),
        17,
    );
}

/// Circle of Protection: Green: "… this turn". Raised in its controller's own turn before anything
/// is dealt, the shield is gone by the opponent's turn, and the green source
/// it was raised against deals its damage in full.
#[test]
fn circle_of_protection_green_shield_ends_with_the_turn() {
    a_circles_shield_ends_with_the_turn(circle_of_protection_green(), wild_elephant(), 17);
}
