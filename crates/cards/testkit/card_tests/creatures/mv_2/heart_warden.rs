//! `cards/creatures/mv_2/heart_warden.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Heart Warden` prints a mana ability `{{T}}: Add {{G}}` and an activated ability `{{2}}, Sacrifice this creature`
/// to draw a card under `Coverage::Implemented`.
/// Tapping mana while excluding `Heart Warden` via `tap_all_mana_but` leaves the creature available to activate
/// its second ability, sacrificing itself to draw a card into hand.
#[test]
fn heart_warden_sacrifices_to_draw() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1630, forest())
        .battlefield(0, &[heart_warden(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let _warden = on_battlefield(&engine, p0, heart_warden()).expect("warden is seated");
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    tap_all_mana_but(&mut engine, p0, Some(heart_warden()));
    assert_eq!(engine.state().players[0].mana_pool.total(), 2);
    assert!(on_battlefield(&engine, p0, heart_warden()).is_some());

    activate(&mut engine, p0, heart_warden(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, heart_warden()).is_some());
    assert!(on_battlefield(&engine, p0, heart_warden()).is_none());
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "a card was drawn from sacrificing Heart Warden"
    );
}
