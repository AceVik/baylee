//! `cards/artifacts/mv_1/sunbeam_spellbomb.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Sunbeam Spellbomb` prints `{{W}}, Sacrifice this artifact: You gain 5 life.` and
/// `{{1}}, Sacrifice this artifact: Draw a card.` with `Coverage::Implemented`.
/// In this scenario, seat 0 controls `Sunbeam Spellbomb` and a `plains()`.
/// Floating one white mana pays for ability 1, sacrificing the spellbomb and gaining 5 life upon resolution.
#[test]
fn sunbeam_spellbomb_sacrifices_to_gain_five_life() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), sunbeam_spellbomb()])
        .life(0, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let _bomb =
        on_battlefield(&engine, p0, sunbeam_spellbomb()).expect("spellbomb is on battlefield");
    tap_all_mana_but(&mut engine, p0, Some(sunbeam_spellbomb()));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        1
    );

    activate(&mut engine, p0, sunbeam_spellbomb(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, sunbeam_spellbomb()).is_some(),
        "`Sunbeam Spellbomb` was sacrificed"
    );
    assert_eq!(engine.state().players[0].life, 25, "gained 5 life");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "white mana was consumed"
    );
}
