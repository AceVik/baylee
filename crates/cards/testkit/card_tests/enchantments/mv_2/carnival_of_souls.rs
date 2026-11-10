//! `cards/enchantments/mv_2/carnival_of_souls.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Carnival of Souls` (`Coverage::Implemented`):
/// "Whenever a creature enters, you lose 1 life and add `{{B}}`."
///
/// Verifies that whenever a creature enters the battlefield, `Carnival of Souls`
/// triggers, causing its controller to lose 1 life and add one black mana.
#[test]
fn carnival_of_souls_triggers_on_creature_entering() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1328, forest())
        .battlefield(0, &[carnival_of_souls(), forest()])
        .hand(0, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    assert_eq!(engine.state().players[0].life, 20);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        0
    );

    cast_from_hand(&mut engine, p0, llanowar_elves());
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0].life,
        19,
        "controller loses 1 life when a creature enters"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        1,
        "controller adds {{B}} when a creature enters"
    );
}
