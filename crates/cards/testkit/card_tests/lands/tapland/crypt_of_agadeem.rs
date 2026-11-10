//! `cards/lands/tapland/crypt_of_agadeem.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Crypt of Agadeem: "This land enters tapped." / "{T}: Add {B}." / "{2}, {T}: Add {B} for each black creature card in your graveyard."
/// With two black creature cards in the graveyard and {2} mana available from two Swamps, ability 1 is activated.
/// The ability counts both black creature cards and adds {B}{B} to the mana pool.
#[test]
fn crypt_of_agadeem_adds_black_mana_per_black_creature_in_graveyard() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(134, sheoldred_the_apocalypse())
        .battlefield(0, &[crypt_of_agadeem(), swamp(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    seed_graveyard(&mut engine, p0, 2);
    let crypt = on_battlefield(&engine, p0, crypt_of_agadeem()).expect("Crypt deployed");

    tap_mana_except(&mut engine, p0, crypt);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        2
    );

    activate(&mut engine, p0, crypt_of_agadeem(), 1);

    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        2,
        "spent 2 black mana to activate, gained 2 black mana from 2 black creature cards"
    );
    assert!(is_tapped(&engine, crypt));
}
