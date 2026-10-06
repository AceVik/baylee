//! `cards/lands/storage/saprazzan_cove.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// The five Mercadian Masques storage lands, which are a cycle: Fountain of
/// Cho, Saprazzan Cove, Subterranean Hangar, Mercadian Bazaar and Rushwood
/// Grove print one text with one symbol changed.
///
/// So this is the reader's test rather than the engine's. The engine sees
/// `Effect::mana_dynamic(<colour>, Amount::X)` five times and cannot tell
/// which colour is right; only the printed text can, and `landgen` is what
/// read it. Five lands banked and spent in one turn each, and the pool
/// afterwards is one of every colour — a cycle member reading the wrong
/// symbol, or all five reading the first one, fails on the count.
#[test]
fn the_storage_cycle_spends_its_counter_for_the_colour_it_prints() {
    let p0 = PlayerId::new(0);
    let cycle = [
        (fountain_of_cho(), ManaColor::White),
        (saprazzan_cove(), ManaColor::Blue),
        (subterranean_hangar(), ManaColor::Black),
        (mercadian_bazaar(), ManaColor::Red),
        (rushwood_grove(), ManaColor::Green),
    ];
    let mut engine = Duel::new(887, forest())
        .battlefield(0, &cycle.map(|(card, _)| card))
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let lands: Vec<ObjectId> = cycle
        .iter()
        .map(|(card, _)| on_battlefield(&engine, p0, *card).expect("the land is on the table"))
        .collect();
    for land in &lands {
        store_a_counter(&mut engine, p0, *land, 0);
        assert_eq!(counters_on(&engine, *land, counters::STORAGE), 1);
    }

    cross_into_the_next_own_main(&mut engine, p0);
    for (land, (_, color)) in lands.iter().zip(cycle) {
        assert_eq!(
            spend_storage(&mut engine, p0, *land, 1, 1),
            (0, 1),
            "one counter each, so one is each land's bound"
        );
        assert_eq!(
            engine.state().players[0].mana_pool.available(color),
            1,
            "and each of them spends it for the symbol it prints"
        );
        assert_eq!(counters_on(&engine, *land, counters::STORAGE), 0);
    }
}
