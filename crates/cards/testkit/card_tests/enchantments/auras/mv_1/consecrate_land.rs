//! `cards/enchantments/auras/mv_1/consecrate_land.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Consecrate Land targets only a
/// land, never the Elves; the land it attaches to gains indestructible,
/// and a bystander land beside it does not.
#[test]
fn consecrate_land_attaches_to_a_land_and_grants_it_indestructible() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), forest(), llanowar_elves()])
        .hand(0, &[consecrate_land()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let target = on_battlefield(&engine, p0, forest()).expect("the Forest is seated");
    let bystander = on_battlefield(&engine, p0, plains()).expect("the Plains is seated");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves is seated");

    attaches_only_to(&mut engine, p0, consecrate_land(), target, elf);
    assert!(
        keywords(&engine, target).contains(KeywordSet::INDESTRUCTIBLE),
        "the enchanted land has indestructible"
    );
    assert!(
        !keywords(&engine, bystander).contains(KeywordSet::INDESTRUCTIBLE),
        "a bystander land beside it does not"
    );
}
