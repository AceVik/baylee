//! `cards/sorceries/mv_4/natural_order.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Natural Order — {2}{G}{G} sorcery: "As an additional cost to cast this
/// spell, sacrifice a green creature. Search your library for a green
/// creature card, put it onto the battlefield, then shuffle."
///
/// Ornithopter stands beside the Elves as the creature that is *not* green:
/// the cost's question must leave it out, which is what separates "a green
/// creature" from "a creature".
#[test]
fn natural_order_sacrifices_a_green_creature_for_a_green_creature_card() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, canopy_spider())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                ornithopter(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[natural_order()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let elves = on_battlefield(&engine, p0, llanowar_elves()).unwrap();
    let library_before = library_size(&engine, p0);

    cast_from_hand(&mut engine, p0, natural_order());
    let offered = sacrifice_as_cast(&mut engine, p0, elves);
    assert_eq!(offered, vec![elves], "Ornithopter is colourless");
    assert!(in_graveyard(&engine, p0, llanowar_elves()).is_some());
    assert!(on_battlefield(&engine, p0, ornithopter()).is_some());

    let Pending::ChooseCards { options, .. } = pass_to_card_choice(&mut engine) else {
        unreachable!("the helper returns only a card choice")
    };
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(fielded(&engine, p0, canopy_spider()), 1);
    assert_eq!(library_size(&engine, p0), library_before - 1);
    assert!(in_graveyard(&engine, p0, natural_order()).is_some());
}
