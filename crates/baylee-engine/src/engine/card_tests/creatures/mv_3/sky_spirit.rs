//! `cards/creatures/mv_3/sky_spirit.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Sky Spirit is `{1}{W}{U}` for a 2/2 Spirit whose whole text is "Flying,
/// first strike", so the card is only itself if that line reaches the
/// permanent the layers project *and* flying actually decides a block. The
/// board carries an untapped ground Elf on each side, which is what makes the
/// block offer readable in both directions: the defending Elf is paired with
/// this seat's own ground attacker, and never with the Spirit — whose two
/// damage then lands on the player that Elf stands beside rather than being
/// soaked up. The keywords come off `characteristics()` because that is the
/// only reading that sees a granted one, and the combat is a turn later
/// because a creature cast this turn is sick (CR 302.6).
#[test]
#[allow(clippy::too_many_lines)] // one play, and both halves of the keyword line read off it
fn sky_spirit_flies_over_a_ground_creature_for_two_damage() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[plains(), island(), island(), quiet_creature()])
        .hand(0, &[sky_spirit()])
        .battlefield(1, &[quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The Elf is the ground creature the flight is measured against, so it is
    // named as the thing kept back: `tap_all_mana` presses every mana ability
    // whose whole price is its own `{T}` (#159), and a creature tapped for
    // mana may not attack afterwards.
    tap_all_mana_but(&mut engine, p0, Some(quiet_creature()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "the three lands pay {{1}}{{W}}{{U}} and the Elf stays standing"
    );
    cast_with_floating(&mut engine, p0, sky_spirit());
    pass_until(&mut engine, stack_is_empty);

    let spirit = on_battlefield(&engine, p0, sky_spirit()).expect("the Spirit resolved");
    let my_elf = on_battlefield(&engine, p0, quiet_creature()).expect("my Elf is out");
    let their_elf = on_battlefield(&engine, p1, quiet_creature()).expect("their Elf is out");
    assert_eq!(pt(&engine, spirit), (2, 2), "the body the card prints");
    let granted = keywords(&engine, spirit);
    assert!(granted.contains(KeywordSet::FLYING), "Flying");
    assert!(granted.contains(KeywordSet::FIRST_STRIKE), "first strike");

    // CR 302.6: a creature cast this turn may not attack, so the combat the
    // keyword line is about is the next turn this seat takes.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing but the attack declaration")
    };
    assert!(
        attackers.contains(&spirit) && attackers.contains(&my_elf),
        "both untapped creatures may attack: {attackers:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (spirit, Defender::Player(p1)),
                    (my_elf, Defender::Player(p1)),
                ],
            },
        )
        .unwrap();

    // The rest of combat, one pending at a time. The defending seat's block
    // offer is where the flight lives, so it is asserted where it is asked
    // rather than inferred from a life total afterwards.
    for _ in 0..80 {
        if matches!(engine.state().turn.phase, Phase::Ending) {
            break;
        }
        match engine.pending().clone() {
            Pending::ChooseBlockers {
                player, blockers, ..
            } => {
                let ground = blockers
                    .iter()
                    .find(|option| option.blocker == their_elf)
                    .unwrap_or_else(|| panic!("the ground Elf may block something: {blockers:?}"));
                assert!(
                    ground.attackers.contains(&my_elf),
                    "the control: two ground 1/1s are a legal pairing: {ground:?}"
                );
                assert!(
                    !ground.attackers.contains(&spirit),
                    "\"Flying\" keeps the ground Elf off the flier: {ground:?}"
                );
                assert!(
                    blockers
                        .iter()
                        .all(|option| !option.attackers.contains(&spirit)),
                    "and no other blocker is paired with it either: {blockers:?}"
                );
                engine
                    .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
                    .expect("declaring no blockers is always legal");
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseAttackers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareAttackers { attackers: vec![] })
                    .unwrap();
            }
            other => panic!("unexpected while the flier's damage resolves: {other:?}"),
        }
    }

    assert_eq!(
        engine.state().players[1].life,
        17,
        "two from the Spirit and one from the ground Elf, and none of it blocked"
    );
    assert!(
        on_battlefield(&engine, p0, sky_spirit()).is_some(),
        "nothing could block the flier, so nothing dealt damage back to it"
    );
    assert!(
        on_battlefield(&engine, p1, quiet_creature()).is_some(),
        "the Elf that was never paired with the flier is still standing"
    );
}
