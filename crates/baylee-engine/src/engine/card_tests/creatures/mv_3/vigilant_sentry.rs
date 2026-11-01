//! `cards/creatures/mv_3/vigilant_sentry.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn vigilant_sentry_gains_plus_one_plus_one_and_tap_to_pump_attacking_or_blocking_creature_at_threshold()
 {
    let sentry = card_index("9bb66d53-a83c-4313-91c3-4df09c248558");
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));

    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[plains(), plains(), plains(), swamp(), savannah_lions()],
        )
        .hand(0, &[sentry, dark_ritual()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    seed_graveyard(&mut engine, p0, 6);
    for p in all_on_battlefield(&engine, p0, plains()) {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source: p })
            .unwrap();
    }
    let card = in_hand(&engine, p0, sentry).unwrap();
    engine.apply(p0, PlayerAction::CastSpell { card }).unwrap();
    pass_until(&mut engine, stack_is_empty);

    let vs = on_battlefield(&engine, p0, sentry).expect("Vigilant Sentry on battlefield");
    let lions = on_battlefield(&engine, p0, savannah_lions()).expect("Savannah Lions");
    assert_eq!(pt(&engine, vs), (2, 2));

    // Cast Dark Ritual as 7th card into graveyard (threshold).
    let s = all_on_battlefield(&engine, p0, swamp())[0];
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: s })
        .unwrap();
    let dr = in_hand(&engine, p0, dark_ritual()).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: dr })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, vs), (3, 3));

    // Advance to P0's next turn so Vigilant Sentry is no longer summoning-sick,
    // attack with Savannah Lions, and tap Vigilant Sentry to give it +3/+3.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    attack_and_collect_blocks(&mut engine, lions, p1);
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: vs,
                ability_index: 0,
            },
        )
        .expect("activate Vigilant Sentry tap ability during declare blockers");
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![lions],
                players: vec![],
            },
        )
        .expect("target attacking Savannah Lions");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, lions), (5, 4));
}
