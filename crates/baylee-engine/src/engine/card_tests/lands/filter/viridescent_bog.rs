//! `cards/lands/filter/viridescent_bog.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Viridescent Bog prints one line — `{1}, {T}: Add {B}{G}` — and the card
/// is entirely in the word "and". A land that tapped for one mana of either
/// colour would be a plausible card and a different one, so the assertion
/// has to read *both* colours out of a single activation rather than a
/// total, which a choice would satisfy just as well.
///
/// The `{1}` is why the Forest is tapped first: `LegalActions` is filtered by
/// `can_afford`, which reads the pool and not the untapped lands, so the
/// ability is on offer only once the green is floating. That same reading is
/// what keeps `tap_all_mana` off the Bog itself — a price containing anything
/// beyond its own `{T}` is a decision this kit will not take on a test's
/// behalf (#159), and `{1}, {T}` is exactly the filter-land shape this
/// prints. One Forest, so the `{1}` consumes the whole pool and the two mana
/// left behind can only be the Bog's own.
#[test]
fn viridescent_bog_spends_a_generic_and_its_tap_for_black_and_green_together() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[viridescent_bog(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bog = on_battlefield(&engine, p0, viridescent_bog()).expect("the Bog is on the table");
    assert!(!is_tapped(&engine, bog), "and it stands untapped");

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the one Forest and nothing else: `{{1}}, {{T}}` is not a cost of \
         exactly its own tap, so the helper left the Bog standing"
    );
    assert!(
        !is_tapped(&engine, bog),
        "which is what lets it pay its own tap below"
    );

    // Ability 0 is the printed `{1}, {T}: Add {B}{G}` — the card's only line.
    activate(&mut engine, p0, viridescent_bog(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Black), 1, "one black");
    assert_eq!(
        pool.available(ManaColor::Green),
        1,
        "and one green out of the same activation — `{{B}}{{G}}` is not a \
         choice between the two"
    );
    assert_eq!(
        pool.total(),
        2,
        "the {{1}} ate the Forest's green, so both mana here are the Bog's \
         own: the add put a black and a green in the pool, not one of them"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so the mana is in the pool \
         the moment the activation is applied"
    );
    assert!(
        is_tapped(&engine, bog),
        "{{T}} was the other half of the cost"
    );
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "and the seat holds priority again, got {:?}",
        engine.pending()
    );
}
