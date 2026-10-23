//! `cards/creatures/mv_2/dauthi_warlord.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);

fn dauthi_warlord() -> CardIndex {
    card_index("42bb6537-fa54-43b7-84f2-dea41adc35fd")
}

fn dauthi_cutthroat() -> CardIndex {
    card_index("5fdde593-54cb-4186-974b-df4383f7efe2")
}

/// Dauthi Warlord (`42bb6537-fa54-43b7-84f2-dea41adc35fd`) — {1}{B} Creature — Dauthi Soldier (Implemented):
/// "Shadow. Dauthi Warlord's power is equal to the number of creatures with shadow on the battlefield."
#[test]
fn dauthi_warlord_power_equals_number_of_creatures_with_shadow_on_battlefield() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[dauthi_warlord()])
        .battlefield(1, &[dauthi_cutthroat()])
        .start();
    assert!(walk_to_own_main(&mut engine, P0));

    let warlord = on_battlefield(&engine, P0, dauthi_warlord()).unwrap();
    assert_eq!(pt(&engine, warlord), (2, 1));
}
