//! `cards/enchantments/mv_4/shimmer.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::subtypes;

fn shimmer() -> CardIndex {
    card_index("f557fb2c-581b-4a8d-8766-1e46e92224fa")
}

/// Shimmer (`{2}{U}{U}`, `Coverage::Implemented`): chooses a land type as it
/// enters and grants each land of the chosen type phasing.
#[test]
fn shimmer_grants_phasing_only_to_lands_of_chosen_type() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(), island(), forest(), forest()])
        .hand(0, &[shimmer()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let my_island = on_battlefield(&engine, p0, island()).unwrap();
    let my_forest = on_battlefield(&engine, p0, forest()).unwrap();

    cast_from_hand(&mut engine, p0, shimmer());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseSubtype { .. })
    });
    engine
        .apply(p0, PlayerAction::ChooseSubtype(subtypes::land::FOREST))
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(keywords(&engine, my_forest).contains(KeywordSet::PHASING));
    assert!(!keywords(&engine, my_island).contains(KeywordSet::PHASING));
}
