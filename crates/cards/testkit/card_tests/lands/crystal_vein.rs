//! `cards/lands/crystal_vein.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Crystal Vein prints two mana abilities: `{T}: Add {C}`, and the line the
/// land is really played for — `{T}, Sacrifice this land: Add {C}{C}`. Two
/// copies stand on the table so both can be read in one turn: one arrived
/// through an actual land drop, and the printing is named to `tap_all_mana_but`
/// because `tap_all_mana` would press the *plain* line on both (its whole price
/// is a `{T}`) and leave nothing untapped to sacrifice.
///
/// Number and price are asserted together, which is the whole of the card: a
/// land that made two colourless and kept standing would be a strictly better
/// Sol Ring, and the pool alone could not tell that from the printing.
#[test]
fn crystal_vein_trades_itself_for_two_colorless_where_its_plain_tap_gives_one() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[crystal_vein()])
        .hand(0, &[crystal_vein()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // One copy arrives the way a land arrives: through the land drop, in the
    // phase it is played. The card prints no enters-tapped clause, so its `{T}`
    // is live on the turn it lands.
    let played = play_land(&mut engine, p0, crystal_vein());
    assert!(
        !is_tapped(&engine, played),
        "the printed land enters untapped"
    );
    let veins = all_on_battlefield(&engine, p0, crystal_vein());
    assert_eq!(veins.len(), 2, "one played and one already on the table");

    // Both printed prices are a `{T}` and, on the second, the land itself —
    // no mana at all — so the offer below is honest with an empty pool, and
    // keeping the printing back is what leaves a land to sacrifice.
    tap_all_mana_but(&mut engine, p0, Some(crystal_vein()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing floats, so every count below is the ability's own"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let mut offered: Vec<u32> = legal
        .abilities
        .iter()
        .filter(|(source, _)| veins.contains(source))
        .map(|(_, index)| *index)
        .collect();
    assert_eq!(
        offered.len(),
        4,
        "two untapped lands, two printed lines each: {offered:?}"
    );
    offered.sort_unstable();
    offered.dedup();
    assert_eq!(offered, vec![0, 1], "and no third line to press");

    // Ability 1 — `{T}, Sacrifice this land: Add {C}{C}`. A mana ability
    // resolves where it is activated (CR 605.3b), so the price is paid and
    // both mana are in the pool the moment the answer is applied.
    activate(&mut engine, p0, crystal_vein(), 1);
    assert!(
        stack_is_empty(&engine),
        "a mana ability never uses the stack"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Colorless),
        2,
        "{{C}}{{C}}, where its plain {{T}} adds one"
    );
    assert_eq!(pool.total(), 2, "and nothing else came with it");
    assert_eq!(
        all_on_battlefield(&engine, p0, crystal_vein()).len(),
        1,
        "the sacrifice was the other half of the price and it was paid"
    );
    assert!(
        in_graveyard(&engine, p0, crystal_vein()).is_some(),
        "and the sacrificed land is in its owner's graveyard"
    );

    // Ability 0, on the copy still standing: one colourless and no permanent,
    // which is what says the two above are the second line's number and not
    // this one's.
    activate(&mut engine, p0, crystal_vein(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Colorless),
        3,
        "one more than the sacrifice line gave: the plain {{T}} adds a single {{C}}"
    );
    let last = all_on_battlefield(&engine, p0, crystal_vein());
    assert_eq!(last.len(), 1, "and it cost no permanent at all");
    assert!(is_tapped(&engine, last[0]), "{{T}} was the whole price");

    // The other half of that price, read on the land that has now spent its
    // tap: the sacrifice line wants an untapped land, so it is not offered.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal
            .abilities
            .iter()
            .any(|(source, index)| *source == last[0] && *index == 1),
        "the sacrifice line wants an untapped land, and this one is tapped"
    );
}
