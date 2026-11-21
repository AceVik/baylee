//! `cards/instants/mv_1/cut_the_earthly_bond.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "127ce189-2758-4789-9e02-4aa71d013431"
fn cut_the_earthly_bond() -> CardIndex {
    card_index("127ce189-2758-4789-9e02-4aa71d013431")
}

/// Cut the Earthly Bond ({U}): targets ONLY an enchanted permanent (excluding
/// unenchanted permanents and the Aura itself) and returns it to its owner's
/// hand.
#[test]
fn cut_the_earthly_bond_bounces_only_enchanted_permanent() {
    let mut engine = lk::game(
        SEED,
        &[],
        &[index::GRIZZLY_BEARS, index::HILL_GIANT],
        &[cut_the_earthly_bond(), index::UNHOLY_STRENGTH],
        &[],
    );
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);
    let giant = lk::obj(&engine, P1, index::HILL_GIANT);

    // Before anything is enchanted, Cut the Earthly Bond has no legal targets.
    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Blue, 1), (ManaColor::Black, 1)],
    );
    assert!(
        !lk::castable(&engine, P0, cut_the_earthly_bond()),
        "cannot cast without an enchanted permanent"
    );

    // Enchant Grizzly Bears with Unholy Strength.
    cast_with_floating(&mut engine, P0, index::UNHOLY_STRENGTH);
    lk::aim(&mut engine, &[bears], &[]);
    lk::settle(&mut engine);
    let aura = lk::obj(&engine, P0, index::UNHOLY_STRENGTH);

    cast_with_floating(&mut engine, P0, cut_the_earthly_bond());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert_eq!(
        options,
        vec![bears],
        "only the enchanted permanent is offered (not unenchanted {giant:?} or Aura {aura:?})"
    );
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    lk::settle(&mut engine);

    assert!(in_hand(&engine, P1, index::GRIZZLY_BEARS).is_some());
    assert!(in_graveyard(&engine, P0, index::UNHOLY_STRENGTH).is_some());
    assert_eq!(on_battlefield(&engine, P1, index::HILL_GIANT), Some(giant));
}
