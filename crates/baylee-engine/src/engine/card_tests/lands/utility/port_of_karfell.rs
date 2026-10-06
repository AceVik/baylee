//! `cards/lands/utility/port_of_karfell.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Port of Karfell prints `This land enters tapped.`, `{{T}}: Add {{U}}.`, and `{{3}}{{U}}{{B}}{{B}}, {{T}}, Sacrifice this land: Mill four cards, then return a creature card from your graveyard to the battlefield tapped.`
///
/// Under `Coverage::Partial`, the reanimated creature enters untapped because `Effect::GraveyardToBattlefield` lacks a destination modifier.
/// Playing Port of Karfell from hand enters tapped.
/// In the following turn, with `{{3}}{{U}}{{B}}{{B}}` floating from basic lands and a creature card in the graveyard, activating ability 1 sacrifices Port of Karfell, mills four cards, and returns the creature card to the battlefield untapped.
#[test]
fn port_of_karfell_enters_tapped_mills_and_reanimates_untapped() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, young_wolf())
        .battlefield(
            0,
            &[swamp(), swamp(), island(), forest(), forest(), forest()],
        )
        .hand(0, &[port_of_karfell()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let karfell = play_land(&mut engine, p0, port_of_karfell());
    assert!(
        entered_tapped(&engine, karfell),
        "port of karfell enters tapped"
    );

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, karfell));

    // Put one creature card into p0's graveyard.
    seed_graveyard(&mut engine, p0, 1);
    let dead_wolf =
        in_graveyard(&engine, p0, young_wolf()).expect("creature card seeded in graveyard");
    let lib_before = library_size(&engine, p0);

    // Float {{3}}{{U}}{{B}}{{B}} from basic lands while keeping Port of Karfell untapped.
    tap_mana_except(&mut engine, p0, karfell);
    assert_eq!(engine.state().players[0].mana_pool.total(), 6);

    activate(&mut engine, p0, port_of_karfell(), 1);

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert!(
        options.contains(&dead_wolf),
        "reanimation ability targets creature in graveyard"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![dead_wolf],
            },
        )
        .unwrap();

    assert!(
        on_battlefield(&engine, p0, port_of_karfell()).is_none(),
        "port of karfell is sacrificed as part of activation cost"
    );
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        library_size(&engine, p0),
        lib_before - 4,
        "milled four cards"
    );
    let wolf =
        on_battlefield(&engine, p0, young_wolf()).expect("reanimated creature is on battlefield");
    assert!(
        !is_tapped(&engine, wolf),
        "reanimated creature enters untapped under `Coverage::Partial`"
    );
}
