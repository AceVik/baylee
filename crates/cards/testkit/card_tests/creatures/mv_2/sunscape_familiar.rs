//! `cards/creatures/mv_2/sunscape_familiar.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

/// Sunscape Familiar — {1}{W} Creature — Wall 0/3
/// "Defender
/// Green spells and blue spells you cast cost {1} less to cast."
#[test]
fn sunscape_familiar_discounts_green_and_blue_spells_you_cast() {
    let familiar = card_index("615603bb-3899-4419-9444-e4b64c94a438");
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[familiar, forest(), island(), island(), island()])
        .hand(0, &[grizzly_bears(), index::PHANTOM_MONSTER])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let familiar_id = on_battlefield(&engine, p0, familiar).unwrap();
    assert!(keywords(&engine, familiar_id).contains(KeywordSet::DEFENDER));

    // Grizzly Bears ({1}{G} -> {G}) and Phantom Monster ({3}{U} -> {2}{U}) cast off 4 lands.
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, grizzly_bears());
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, grizzly_bears()).is_some());

    cast_with_floating(&mut engine, p0, index::PHANTOM_MONSTER);
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, index::PHANTOM_MONSTER).is_some());
}
