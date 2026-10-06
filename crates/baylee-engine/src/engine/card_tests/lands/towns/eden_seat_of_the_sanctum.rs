//! `cards/lands/towns/eden_seat_of_the_sanctum.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Eden, Seat of the Sanctum prints `{T}: Add {C}` and `{5}, {T}: Mill two cards. Then you may sacrifice
/// this land. When you do, return another target permanent card from your graveyard to your hand.`
/// With five mana floating both abilities are offered, and the first still adds exactly one `{C}`.
/// The second, whose return is a reflexive trigger (CR 603.12), is played end to end in
/// `reflexive_tests`: its target is chosen after the mill, declining creates nothing, and a target
/// exiled in response is not returned.
#[test]
fn eden_seat_of_the_sanctum_taps_for_colorless_and_offers_its_mill_ability_at_five() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest(), forest()])
        .hand(0, &[eden_seat_of_the_sanctum()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, eden_seat_of_the_sanctum());
    assert!(!is_tapped(&engine, land));

    tap_all_mana_but(&mut engine, p0, Some(eden_seat_of_the_sanctum()));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        5
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority");
    };
    let offered: Vec<(ObjectId, u32)> = legal
        .abilities
        .iter()
        .copied()
        .filter(|(source, _)| *source == land)
        .collect();
    assert_eq!(offered, vec![(land, 0), (land, 1)]);

    activate(&mut engine, p0, eden_seat_of_the_sanctum(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert_eq!(pool.total(), 6);
    assert!(is_tapped(&engine, land));
}
