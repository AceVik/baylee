//! `cards/artifacts/mv_4/conservator.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Conservator: "{3}, {T}: Prevent the next 2 damage that would be dealt to
/// you this turn." A shield of exactly 2, read off a burn spell for 3: one
/// point gets through, proving the shield absorbs its printed amount and
/// nothing more (`prevention::ShieldKind::Next`).
#[test]
fn conservator_prevents_the_next_2_damage_dealt_to_its_controller() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[conservator(), mountain(), forest(), forest(), forest()],
        )
        .hand(0, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let red = on_battlefield(&engine, p0, mountain()).expect("the Mountain is out");
    tap_mana_except(&mut engine, p0, red);
    activate(&mut engine, p0, conservator(), 0);
    pass_until(&mut engine, stack_is_empty);

    let before = life_of(&engine, p0);
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, lightning_bolt());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .expect("p0 is any target");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        life_of(&engine, p0),
        before - 1,
        "3 dealt, 2 prevented by the shield"
    );
}
