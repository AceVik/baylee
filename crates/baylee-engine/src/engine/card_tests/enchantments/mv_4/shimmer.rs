//! `cards/enchantments/mv_4/shimmer.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
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

/// "Each land of the chosen type", whoever controls it: the opponent's Forest
/// phases out at its controller's untap step, the controller's Island (not the
/// chosen type) does not.
#[test]
fn shimmer_phases_an_opponents_land_of_the_chosen_type_and_spares_other_lands() {
    const P0: PlayerId = PlayerId::new(0);
    const P1: PlayerId = PlayerId::new(1);
    let mut engine = lk::game(SEED, &[island()], &[forest()], &[shimmer()], &[]);
    let my_island = lk::obj(&engine, P0, island());
    let their_forest = lk::obj(&engine, P1, forest());

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 2), (ManaColor::Blue, 2)],
    );
    cast_with_floating(&mut engine, P0, shimmer());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseSubtype { .. })
    });
    engine
        .apply(P0, PlayerAction::ChooseSubtype(subtypes::land::FOREST))
        .unwrap();
    lk::settle(&mut engine);
    assert!(keywords(&engine, their_forest).contains(KeywordSet::PHASING));
    assert!(!keywords(&engine, my_island).contains(KeywordSet::PHASING));

    reach_their_main_phase(&mut engine, P1);
    let phased_out = |id| {
        engine
            .state()
            .object(id)
            .unwrap()
            .status
            .contains(Status::PHASED_OUT)
    };
    assert!(
        phased_out(their_forest),
        "phased out before its controller untapped"
    );
    assert!(!phased_out(my_island));
}
