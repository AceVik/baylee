//! `cards/lands/storage/dreadship_reef.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// The five Time Spiral storage lands, which print one text with one pair of
/// symbols changed.
///
/// The engine sees `Effect::mana_combination(&[_, _], Amount::X)` five times
/// and cannot tell a right pair from a wrong one; `Pending::ChooseColor`'s
/// options are `landgen`'s emitted list handed straight back, so this is the
/// reader on trial. Two lands sharing a pair, or all five reading the first
/// one, fails here.
#[test]
fn the_time_spiral_storage_cycle_offers_the_pair_each_land_prints() {
    let p0 = PlayerId::new(0);
    let cycle = [
        (calciform_pools(), [ManaColor::White, ManaColor::Blue]),
        (dreadship_reef(), [ManaColor::Blue, ManaColor::Black]),
        (molten_slagheap(), [ManaColor::Black, ManaColor::Red]),
        (fungal_reaches(), [ManaColor::Red, ManaColor::Green]),
        (saltcrusted_steppe(), [ManaColor::Green, ManaColor::White]),
    ];
    let mut board: Vec<CardIndex> = cycle.iter().map(|(card, _)| *card).collect();
    board.extend(std::iter::repeat_n(forest(), 5));
    let mut engine = Duel::new(917, forest()).battlefield(0, &board).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let lands: Vec<ObjectId> = cycle
        .iter()
        .map(|(card, _)| on_battlefield(&engine, p0, *card).expect("the land is on the table"))
        .collect();

    // Five Forests bank five counters, one per land.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    for source in legal.mana_abilities.clone() {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    for land in &lands {
        store_a_counter(&mut engine, p0, *land, 1);
        assert_eq!(counters_on(&engine, *land, counters::STORAGE), 1);
    }

    // And the next turn spends them, five more Forests paying the five {1}s.
    cross_into_the_next_own_main(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    for source in legal.mana_abilities.clone() {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    for (land, (_, pair)) in lands.iter().zip(cycle) {
        assert_eq!(
            spend_storage(&mut engine, p0, *land, 2, 1),
            (0, 1),
            "one counter each, so one is each land's bound"
        );
        let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
            panic!("one counter buys one pick: {:?}", engine.pending())
        };
        assert_eq!(
            options,
            pair.to_vec(),
            "each land offers the pair it prints and no other"
        );
        // The delta and not the total, because the `{1}` of the *next*
        // activation is paid out of the same pool and may well be paid with
        // the mana this one just made — which is a legal thing for a player
        // to do and would make a sweep at the end read the wrong number for
        // a reason that has nothing to do with the cycle.
        let before = engine.state().players[0].mana_pool.available(pair[1]);
        engine
            .apply(p0, PlayerAction::ChooseColor(pair[1]))
            .expect("a colour the engine offered");
        assert_eq!(
            engine.state().players[0].mana_pool.available(pair[1]),
            before + 1,
            "one counter, one mana, of the colour that was picked"
        );
        assert_eq!(counters_on(&engine, *land, counters::STORAGE), 0);
    }
}
