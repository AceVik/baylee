//! `cards/creatures/mv_5/zirilan_of_the_claw.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn zirilan_of_the_claw() -> CardIndex {
    card_index("3aeb28ac-8ed2-4a90-a631-d84361ac6acc")
}

/// Zirilan of the Claw (`{3}{R}{R}`, `Coverage::Partial`): only the 3/4 body is
/// played. The Dragon search is stripped because nothing can give "that
/// Dragon" haste or exile it at the next end step, and a search without them
/// would leave a Dragon on the battlefield for good: the ability must not be
/// offered, activating it is refused, and the library's Dragon stays put.
#[test]
fn zirilan_of_the_claw_is_a_3_4_body_that_cannot_fetch_a_dragon() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, shivan_dragon())
        .battlefield(
            0,
            &[zirilan_of_the_claw(), mountain(), mountain(), mountain()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    let zirilan = on_battlefield(&engine, p0, zirilan_of_the_claw()).unwrap();
    assert_eq!(pt(&engine, zirilan), (3, 4), "the printed body");
    tap_all_mana(&mut engine, p0);
    assert!(
        !ability_offered(&engine, zirilan, 0),
        "the Dragon search is not played"
    );
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: zirilan,
                    ability_index: 0,
                },
            )
            .is_err(),
        "there is no ability to activate"
    );
    assert!(
        on_battlefield(&engine, p0, shivan_dragon()).is_none(),
        "no Dragon came out of the library"
    );
    assert!(!is_tapped(&engine, zirilan), "nothing was activated");
}
