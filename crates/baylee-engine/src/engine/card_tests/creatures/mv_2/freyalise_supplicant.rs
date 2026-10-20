//! `cards/creatures/mv_2/freyalise_supplicant.rs`, played.
//!
//! Oracle:
//! - "{T}, Sacrifice a red or white creature: This creature deals damage to any target equal to half the sacrificed creature's power, rounded down."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn freyalise_supplicant() -> CardIndex {
    card_index("c28a7641-49e3-43b1-a366-8d6e9c8dab9c")
}

#[test]
fn freyalise_supplicant_sacrifices_red_or_white_creature_for_half_power_damage_rounded_down() {
    let mut engine = lk::game(
        SEED,
        &[freyalise_supplicant(), index::SHIVAN_DRAGON],
        &[],
        &[],
        &[],
    );
    let dragon = lk::obj(&engine, P0, index::SHIVAN_DRAGON);

    activate(&mut engine, P0, freyalise_supplicant(), 0);
    lk::aim(&mut engine, &[], &[P1]);
    lk::drive(&mut engine, &[dragon], true, 0);
    lk::settle(&mut engine);

    assert_eq!(lk::zone_of(&engine, dragon), Zone::Graveyard);
    assert_eq!(
        lk::life(&engine, P1),
        18,
        "half of 5 power rounded down deals 2 damage"
    );
}
