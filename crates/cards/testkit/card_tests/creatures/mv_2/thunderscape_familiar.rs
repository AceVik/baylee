//! `cards/creatures/mv_2/thunderscape_familiar.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

/// Thunderscape Familiar — {1}{R} Creature — Kavu 1/1
/// "First strike
/// Black spells and green spells you cast cost {1} less to cast."
#[test]
fn thunderscape_familiar_discounts_black_and_green_spells_you_cast() {
    let familiar = card_index("40f6ea6f-71e1-46bd-8d9a-ccb5f437892c");
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[familiar, swamp(), swamp(), swamp(), forest()])
        .hand(0, &[index::BOG_WRAITH, grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let familiar_id = on_battlefield(&engine, p0, familiar).unwrap();
    assert!(keywords(&engine, familiar_id).contains(KeywordSet::FIRST_STRIKE));

    // Bog Wraith ({3}{B} -> {2}{B}) and Grizzly Bears ({1}{G} -> {G}) cast off 4 lands.
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, index::BOG_WRAITH);
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, index::BOG_WRAITH).is_some());

    cast_with_floating(&mut engine, p0, grizzly_bears());
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, grizzly_bears()).is_some());
}
