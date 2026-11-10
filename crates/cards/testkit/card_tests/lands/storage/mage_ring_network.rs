//! `cards/lands/storage/mage_ring_network.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Mage-Ring Network, which prints three abilities where the five Mercadian
/// Masques lands print two: `{T}: Add {C}.`, `{1}, {T}: Put a storage counter
/// on this land.` and the storage line itself. Seven of the pool's seventeen
/// counter-X lands have three ability lines — the five Time Spiral ones and
/// Crucible of the Spirit Dragon are the rest — so this is a shape rather
/// than a card; it is simply the only one of them the engine can play today.
///
/// It is the discriminating card for **where the question comes from**.
/// Fountain of Cho alone cannot tell "this ability announces a number" from
/// "this card announces a number", because every ability it prints that
/// could ask does ask. Here the same permanent carries a plain mana ability
/// and a storage line, so a question attached to the source rather than to
/// `CostPart::RemoveCounterSelfX` would fire on `{T}: Add {C}` too — and
/// that is asserted directly.
///
/// The `{1}` is the other half. A cost with a mana part *and* an announced
/// number is the shape that would break an implementation that took the X
/// question as the whole of the cost: the generic mana still has to come out
/// of the pool, and it does, one Forest's worth.
#[test]
fn only_the_storage_line_of_mage_ring_network_asks_for_a_number() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(883, forest())
        .battlefield(0, &[mage_ring_network(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let net = on_battlefield(&engine, p0, mage_ring_network()).expect("the land is on the table");

    // `{1}, {T}: Put a storage counter on this land.` — the Forest pays the
    // generic, and the land is the one thing that must not.
    tap_mana_except(&mut engine, p0, net);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1,
        "one Forest, floating"
    );
    store_a_counter(&mut engine, p0, net, 1);
    assert_eq!(counters_on(&engine, net, counters::STORAGE), 1);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        0,
        "and the {{1}} really was paid out of the pool"
    );

    // `{T}: Add {C}.` — the same permanent, a counter on it, and no question.
    cross_into_the_next_own_main(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: net,
                ability_index: 0,
            },
        )
        .expect("a printed mana ability of an untapped land");
    assert!(
        matches!(engine.pending(), Pending::Priority { .. }),
        "the number belongs to the cost that removes counters, not to the \
         land that has some: {:?}",
        engine.pending()
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        1,
        "one colourless, and none of it came from the counter"
    );
    assert_eq!(
        counters_on(&engine, net, counters::STORAGE),
        1,
        "which is still sitting there untouched"
    );

    // And the storage line, which does ask.
    cross_into_the_next_own_main(&mut engine, p0);
    assert_eq!(
        spend_storage(&mut engine, p0, net, 2, 1),
        (0, 1),
        "one banked counter is a bound of one"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        1,
        "`Add {{C}} for each storage counter removed this way`"
    );
    assert_eq!(counters_on(&engine, net, counters::STORAGE), 0);
}
