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

    // Gray Ogre does not have to attack and survives P0's end step without attacking.
    reach_their_main_phase(&mut engine, P1);
    assert!(
        on_battlefield(&engine, P0, gray_ogre()).is_some(),
        "paying {{X}} prevented both the attack requirement and end-step destruction"
    );
}
