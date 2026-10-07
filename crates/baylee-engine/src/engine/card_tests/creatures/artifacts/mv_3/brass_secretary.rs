//! `cards/creatures/artifacts/mv_3/brass_secretary.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Brass Secretary` prints `{{2}}, Sacrifice this creature: Draw a card.` with `Coverage::Implemented`.
/// With two `forest()` lands floating two mana into seat 0's mana pool, activating the ability
/// pays the `{2}` mana and sacrifices the creature. Resolving the ability draws a card,
/// increasing the hand size by one while `Brass Secretary` rests in the graveyard.
#[test]
fn brass_secretary_pays_two_mana_and_sacrifices_to_draw_a_card() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), brass_secretary()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let initial_hand = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    tap_all_mana(&mut engine, p0);
    assert_eq!(engine.state().players[0].mana_pool.total(), 2);

    activate(&mut engine, p0, brass_secretary(), 0);
    assert!(
        on_battlefield(&engine, p0, brass_secretary()).is_none(),
        "`Brass Secretary` is sacrificed upon activation"
    );
    assert!(in_graveyard(&engine, p0, brass_secretary()).is_some());
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the two mana floating paid the activation cost"
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        initial_hand + 1,
        "resolving the ability drew one card"
    );
}
