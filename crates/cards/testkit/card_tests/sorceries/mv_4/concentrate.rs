//! `cards/sorceries/mv_4/concentrate.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Concentrate is `{2}{U}{U}` for one sentence: "Draw three cards." The draw is
/// the whole card, so the three cards are named *before* anything is cast — the
/// top of p0's library, which the zone lists bottom first — and read back in
/// hand afterwards, because a hand that merely grew by three could not say
/// which cards moved. Four Islands pay `{2}{U}{U}` to the last mana, so the
/// emptied pool afterwards says the price was charged and not just printed, and
/// the sorcery's own card is read in the graveyard rather than assumed to have
/// resolved.
#[test]
fn concentrate_draws_the_top_three_cards_for_its_four_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island()])
        .hand(0, &[concentrate()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The three cards the draw is about, named while they are still in the
    // library: the zone lists its bottom first, so the top three are its last
    // three entries, topmost last.
    let library_before = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    let top_three: Vec<ObjectId> = library_before.iter().rev().take(3).copied().collect();
    assert_eq!(top_three.len(), 3, "p0 has a library to draw from");
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    // `legal.castable` is filtered through `can_afford`, and that reads the
    // pool rather than the four untapped Islands: with nothing floating the
    // {2}{U}{U} is unpayable, so the sorcery is not offered at all.
    let card = in_hand(&engine, p0, concentrate()).expect("the sorcery is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&card),
        "an empty pool pays no {{2}}{{U}}{{U}}, so the sorcery is not offered: {:?}",
        legal.castable
    );

    tap_all_mana(&mut engine, p0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Blue),
        4,
        "four Islands, four blue"
    );
    assert_eq!(pool.total(), 4, "and nothing else on the board makes mana");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&card),
        "with four blue floating the whole cost is payable: {:?}",
        legal.castable
    );

    cast_with_floating(&mut engine, p0, concentrate());
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{2}}{{U}}{{U}} came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "a sorcery uses the stack, so the draw has not happened yet"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, concentrate()).is_some(),
        "the sorcery resolved and went to its owner's graveyard"
    );
    for (slot, id) in top_three.iter().enumerate() {
        assert!(
            engine
                .state()
                .zones
                .list(ZoneLocation::Hand(p0))
                .contains(id),
            "card {slot} from the top of the library is in hand, not merely \
             gone from the library"
        );
    }
    assert_eq!(
        library_size(&engine, p0),
        library_before.len() - 3,
        "\"Draw three cards\": three cards left the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 2,
        "the spell left the hand and three cards arrived — three drawn, one cast"
    );
    assert!(
        all_on_battlefield(&engine, p0, island())
            .iter()
            .all(|id| is_tapped(&engine, *id)),
        "the four Islands are down: the mana the cast spent was theirs"
    );
}
