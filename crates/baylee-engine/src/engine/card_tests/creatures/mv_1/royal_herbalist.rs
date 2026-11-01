//! `cards/creatures/mv_1/royal_herbalist.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w2c_kit::life;

fn royal_herbalist() -> CardIndex {
    card_index("b3baf498-3be7-4a22-8fed-806d8d8ac748")
}

/// Royal Herbalist ({W} 1/1 Creature — Human Cleric, Partial):
/// "{2}, Exile the top card of your library: You gain 1 life." The cost has
/// no `CostPart`, and the ability without it would be a free life gain, so the
/// card offers none. A pinned limitation: this test moves when the cost is
/// said.
#[test]
fn royal_herbalist_offers_no_life_gain_without_its_cost() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[royal_herbalist(), plains(), plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let herbalist = on_battlefield(&engine, p0, royal_herbalist()).unwrap();
    let life_before = life(&engine, p0);
    let library_before = engine.state().zones.list(ZoneLocation::Library(p0)).len();
    tap_all_mana(&mut engine, p0);

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("priority, got {:?}", engine.pending())
    };
    assert!(
        !legal
            .abilities
            .iter()
            .any(|(source, _)| *source == herbalist),
        "nothing is offered on the Herbalist"
    );
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: herbalist,
                    ability_index: 0,
                },
            )
            .is_err(),
        "and nothing can be activated"
    );
    assert_eq!(life(&engine, p0), life_before);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Library(p0)).len(),
        library_before
    );
}
