//! `cards/creatures/mv_2/thornscape_familiar.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

/// Thornscape Familiar — {1}{G} Creature — Insect 2/1
/// "Red spells and white spells you cast cost {1} less to cast."
#[test]
fn thornscape_familiar_discounts_red_and_white_spells_you_cast() {
    let familiar = card_index("3f92c684-f2fd-4fe5-bfb5-e27061d2c45b");
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                familiar,
                mountain(),
                mountain(),
                mountain(),
                plains(),
                plains(),
                plains(),
                plains(),
            ],
        )
        .hand(0, &[index::HILL_GIANT, index::SERRA_ANGEL])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Hill Giant ({3}{R} -> {2}{R}) and Serra Angel ({3}{W}{W} -> {2}{W}{W}) cast off 7 lands.
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, index::HILL_GIANT);
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, index::HILL_GIANT).is_some());

    cast_with_floating(&mut engine, p0, index::SERRA_ANGEL);
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, index::SERRA_ANGEL).is_some());
}
