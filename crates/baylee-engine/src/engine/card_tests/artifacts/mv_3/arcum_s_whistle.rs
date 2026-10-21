//! `cards/artifacts/mv_3/arcum_s_whistle.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::turn::Phase;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn arcum_s_whistle() -> CardIndex {
    card_index("de691eed-d90b-48f9-bae4-f982767f2bfd")
}

fn wall_of_wood() -> CardIndex {
    card_index("aa5e6894-8c97-46f8-a0fd-07c6db51e9a7")
}

/// Arcum's Whistle (de691eed-d90b-48f9-bae4-f982767f2bfd) — {3} Artifact:
/// - "{3}, {T}: Choose target non-Wall creature the active player has controlled
///   continuously since the beginning of the turn. That player may pay {X}, where
///   X is that creature's mana value. If they don't pay, the creature attacks
///   this turn if able, and at the beginning of the next end step, destroy it if
///   it didn't attack this turn. Activate only before attackers are declared."
#[test]
fn arcum_s_whistle_when_unpaid_forces_attack_or_destroys_at_end_step() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                arcum_s_whistle(),
                forest(),
                forest(),
                forest(),
                forest(),
                gray_ogre(),
                wall_of_wood(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let ogre = on_battlefield(&engine, P0, gray_ogre()).expect("ready Ogre");
    let wall = on_battlefield(&engine, P0, wall_of_wood()).expect("Wall of Wood");
    let their_elf = on_battlefield(&engine, P1, llanowar_elves()).expect("inactive player's Elf");

    // Cast a fresh Llanowar Elves so it has summoning sickness (not controlled
    // continuously since the beginning of the turn).
    cast_from_hand(&mut engine, P0, llanowar_elves());
    pass_until(&mut engine, stack_is_empty);
    let sick_elf = on_battlefield(&engine, P0, llanowar_elves()).expect("summoning-sick Elf");

    tap_all_mana_but(&mut engine, P0, Some(llanowar_elves()));
    activate(&mut engine, P0, arcum_s_whistle(), 0);

    let menu = aim_at(&mut engine, P0, ogre);
    assert!(
        menu.contains(&ogre),
        "ready non-Wall active-player creature is legal"
    );
    assert!(!menu.contains(&wall), "Walls are excluded");
    assert!(
        !menu.contains(&sick_elf),
        "creatures that entered this turn are excluded"
    );
    assert!(
        !menu.contains(&their_elf),
        "non-active player's creatures are excluded"
    );

    // Resolve ability -> prompts controller to pay {X} (where X = 3 for Gray Ogre).
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine
        .apply(P0, PlayerAction::YesNo(false))
        .expect("decline to pay {3}");
    pass_until(&mut engine, stack_is_empty);

    // Advance to declare attackers: refusing to attack with Gray Ogre is illegal
    // because it must attack this turn if able.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    assert!(
        engine
            .apply(P0, PlayerAction::DeclareAttackers { attackers: vec![] })
            .is_err(),
        "Gray Ogre must attack this turn if able"
    );
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(ogre, Defender::Player(P1))],
            },
        )
        .expect("attacking with Gray Ogre satisfies the requirement");
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(engine.state().players[1].life, 18);

    // Because Gray Ogre attacked this turn, it is not destroyed at the end step.
    reach_their_main_phase(&mut engine, P1);
    assert!(on_battlefield(&engine, P0, gray_ogre()).is_some());
}

#[test]
fn arcum_s_whistle_when_paid_does_not_force_attack_or_destroy_at_end_step() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                arcum_s_whistle(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                gray_ogre(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let ogre = on_battlefield(&engine, P0, gray_ogre()).expect("ready Ogre");

    // Tap all 6 Forests -> 3 mana pays for Whistle activation, 3 mana stays floating to pay {X}.
    tap_all_mana_but(&mut engine, P0, None);
    activate(&mut engine, P0, arcum_s_whistle(), 0);
    aim_at(&mut engine, P0, ogre);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine
        .apply(P0, PlayerAction::YesNo(true))
        .expect("pay {3} for Gray Ogre's mana value");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the three floating mana paid X = 3"
    );

    // Gray Ogre does not have to attack and survives P0's end step without attacking.
    reach_their_main_phase(&mut engine, P1);
    assert!(
        on_battlefield(&engine, P0, gray_ogre()).is_some(),
        "paying {{X}} prevented both the attack requirement and end-step destruction"
    );
}

/// "If they don't pay, ... at the beginning of the next end step, destroy it
/// if it didn't attack this turn": unpaid, and the Ogre tapped before
/// attackers are declared, so it cannot attack and the requirement asks
/// nothing of the declaration; the delayed trigger destroys it at P0's end
/// step.
#[test]
fn arcum_s_whistle_when_unpaid_destroys_a_creature_that_did_not_attack() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[arcum_s_whistle(), forest(), forest(), forest(), gray_ogre()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let ogre = on_battlefield(&engine, P0, gray_ogre()).expect("ready Ogre");

    tap_all_mana_but(&mut engine, P0, None);
    activate(&mut engine, P0, arcum_s_whistle(), 0);
    aim_at(&mut engine, P0, ogre);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine
        .apply(P0, PlayerAction::YesNo(false))
        .expect("decline to pay {3}");
    pass_until(&mut engine, stack_is_empty);

    engine
        .dev_state_mut(P0)
        .unwrap()
        .object_mut(ogre)
        .unwrap()
        .status
        .insert(Status::TAPPED);
    engine.refresh_offer();

    reach_their_main_phase(&mut engine, P1);
    assert!(
        on_battlefield(&engine, P0, gray_ogre()).is_none(),
        "the Ogre did not attack this turn and was destroyed at the next end step"
    );
    assert!(in_graveyard(&engine, P0, gray_ogre()).is_some());
}

/// "That player may pay {X}": the active player, who controls the target,
/// not the Whistle's controller. P0 activates in P1's first main phase: its
/// own Elves are no target (P0 is not the active player), P1's Ogre is, and
/// P1 is asked. Unpaid, P1's Ogre must attack.
#[test]
fn arcum_s_whistle_asks_the_active_player_who_controls_the_creature() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                arcum_s_whistle(),
                forest(),
                forest(),
                forest(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, P1);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P0),
    );

    let their_ogre = on_battlefield(&engine, P1, gray_ogre()).expect("their Ogre");
    let my_elf = on_battlefield(&engine, P0, llanowar_elves()).expect("my Elves");

    tap_all_mana_but(&mut engine, P0, Some(llanowar_elves()));
    activate(&mut engine, P0, arcum_s_whistle(), 0);
    let menu = aim_at(&mut engine, P0, their_ogre);
    assert!(
        !menu.contains(&my_elf),
        "a creature of the nonactive player is no target: {menu:?}"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    let Pending::YesNo { player, .. } = engine.pending().clone() else {
        unreachable!("the pass waited for exactly this")
    };
    assert_eq!(player, P1, "the active player decides whether to pay");
    engine
        .apply(P1, PlayerAction::YesNo(false))
        .expect("P1 declines to pay {3}");
    pass_until(&mut engine, stack_is_empty);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    assert!(
        engine
            .apply(P1, PlayerAction::DeclareAttackers { attackers: vec![] })
            .is_err(),
        "unpaid, P1's Ogre attacks this turn if able"
    );
}

/// "Activate only before attackers are declared": offered in the first main
/// phase, and no longer in the second, attack or no attack.
#[test]
fn arcum_s_whistle_is_not_offered_once_the_declare_attackers_step_has_passed() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[arcum_s_whistle(), forest(), forest(), forest(), gray_ogre()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let whistle = on_battlefield(&engine, P0, arcum_s_whistle()).expect("Whistle seated");

    assert!(
        ability_offered(&engine, whistle, 0),
        "offered before combat"
    );
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert!(
        !ability_offered(&engine, whistle, 0),
        "not after the declare attackers step"
    );
}
