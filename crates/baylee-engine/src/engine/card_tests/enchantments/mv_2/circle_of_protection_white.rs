//! `cards/enchantments/mv_2/circle_of_protection_white.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Circle of Protection: White, played the same way against a white
/// attacker (Ondu Cleric).
#[test]
fn circle_of_protection_white_prevents_damage_from_a_chosen_white_attacker() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let cop = circle_of_protection_white();
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[cop, forest()])
        .battlefield(1, &[ondu_cleric()])
        .start();
    keep_mulligans(&mut engine);
    let cleric = on_battlefield(&engine, p1, ondu_cleric()).expect("their Cleric");
    reach_their_main_phase(&mut engine, p1);

    let blockers = attack_and_collect_blocks(&mut engine, cleric, p0);
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
        object: cleric,
        version: engine
            .state()
            .object(cleric)
            .expect("source exists")
            .version,
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
    // Circle of Protection: White is itself a white permanent ({1}{W}), so
    // it is also a legal (if useless) choice of "white source"; the Cleric
    // is the one that matters.
    assert!(
        options.contains(&selected),
        "the Cleric is an offered white source: {options:?}"
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
        "the Cleric's combat damage was prevented"
    );
}

/// Circle of Protection: White: "The next time a white source of your choice would deal damage to
/// you this turn, prevent that damage." Three attackers: the double striker chosen has its first blow prevented and its second dealt (1), the Cleric deals 1 and the red Ogre 2.
/// The offer holds both white sources and not the Ogre, and the shield is
/// spent by the one hit it was for.
#[test]
fn circle_of_protection_white_prevents_one_chosen_source_once_and_only_white_ones_are_offered() {
    a_circle_prevents_the_next_damage_of_one_chosen_source(
        circle_of_protection_white(),
        skyhunter_skirmisher(),
        ondu_cleric(),
        gray_ogre(),
        16,
    );
}

/// Circle of Protection: White: "… this turn". Raised in its controller's own turn before anything
/// is dealt, the shield is gone by the opponent's turn, and the white source
/// it was raised against deals its damage in full.
#[test]
fn circle_of_protection_white_shield_ends_with_the_turn() {
    a_circles_shield_ends_with_the_turn(circle_of_protection_white(), skyhunter_skirmisher(), 18);
}
