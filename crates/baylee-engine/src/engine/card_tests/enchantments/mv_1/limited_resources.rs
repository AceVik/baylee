//! `cards/enchantments/mv_1/limited_resources.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn limited_resources() -> CardIndex {
    card_index("5b287dcc-f853-4df1-8ddf-5b8b2adc2873")
}

#[test]
fn limited_resources_resolves_onto_the_battlefield() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1, forest())
        .battlefield(0, &[plains()])
        .hand(0, &[limited_resources()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    cast_from_hand(&mut engine, p0, limited_resources());
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, limited_resources()).is_some());
}
