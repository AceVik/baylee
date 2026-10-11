//! `cards/lands/utility/terrain_generator.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Terrain Generator: "{T}: Add {C}." / "{2}, {T}: You may put a basic land card from your hand onto the battlefield tapped."
/// Under `Coverage::Partial`, the hand-to-battlefield basic land drop ability is omitted.
/// The land taps for its base mana ability to add {C} to the mana pool.
#[test]
fn terrain_generator_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(122, forest())
        .battlefield(0, &[terrain_generator()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let generator = on_battlefield(&engine, p0, terrain_generator()).expect("Generator deployed");
    activate(&mut engine, p0, terrain_generator(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1);
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, generator));
}

#[test]
fn terrain_generator_puts_only_a_basic_land_tapped_without_spending_a_land_drop() {
    use baylee_core::generated::index;
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[index::TERRAIN_GENERATOR])
        .hand(0, &[index::FOREST, index::CITY_OF_BRASS])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let basic = in_hand(&e, p0, index::FOREST).unwrap();
    let nonbasic = in_hand(&e, p0, index::CITY_OF_BRASS).unwrap();
    e.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Colorless, 2);
    e.refresh_offer();
    activate(&mut e, p0, index::TERRAIN_GENERATOR, 1);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = e.pending() else {
        panic!("choice")
    };
    assert!(options.contains(&basic));
    assert!(!options.contains(&nonbasic));
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![basic],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert!(is_tapped(&e, basic));
    assert_eq!(e.state().players[0].lands_played_this_turn, 0);
    play_land(&mut e, p0, index::CITY_OF_BRASS);
    assert_eq!(e.state().players[0].lands_played_this_turn, 1);
}
