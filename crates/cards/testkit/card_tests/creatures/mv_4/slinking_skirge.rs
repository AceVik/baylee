//! `cards/creatures/mv_4/slinking_skirge.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Slinking Skirge — {3}{B}, a 2/1 Phyrexian Imp with flying and one printed
/// line: "{2}, Sacrifice this creature: Draw a card." Both halves of that price
/// leave a mark a test can read, and each needs its own control: the {2} is read
/// against an empty pool, where the ability is not merely refused but absent
/// from the offer, because `can_afford` reads the pool and not the untapped
/// lands; the sacrifice is read as the card sitting in its owner's graveyard
/// the moment the activation lands, with the draw still waiting on the stack.
/// Four Swamps pay the cast down to the last mana, so the first reading is a
/// claim about the ability rather than about a leftover, and the four that
/// untap a turn later are what actually buys the card in hand.
#[test]
#[allow(clippy::too_many_lines)]
fn slinking_skirge_trades_itself_for_a_card_only_once_its_two_mana_is_paid() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(); 4])
        .hand(0, &[slinking_skirge()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Nothing floats and four untapped Swamps are standing: `can_afford` reads
    // the pool, so the {3}{B} is unpaid and the creature is not yet castable.
    let card = in_hand(&engine, p0, slinking_skirge()).expect("the Skirge is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal.castable.contains(&card),
        "an empty pool pays no {{3}}{{B}} whatever the untapped lands say: {:?}",
        legal.castable
    );

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Swamps, four black, and the Skirge is not on the battlefield yet"
    );
    cast_with_floating(&mut engine, p0, slinking_skirge());
    pass_until(&mut engine, |e| at_rest(e, p0));

    let skirge = on_battlefield(&engine, p0, slinking_skirge()).expect("the Skirge resolved");
    assert_eq!(pt(&engine, skirge), (2, 1), "the printed 2/1 body");
    assert!(
        keywords(&engine, skirge).contains(KeywordSet::FLYING),
        "the printed flying reaches the permanent"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "four Swamps paid the {{3}}{{B}} to the last mana"
    );

    // The empty pool is the control for the printed price: the ability charges
    // {2}, so with nothing floating it is not offered at all.
    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert_eq!(player, p0, "the seat with the Skirge holds priority");
    assert!(
        !legal.abilities.iter().any(|(id, _)| *id == skirge),
        "{{2}} is not two, so the whole price is unpayable and nothing is \
         offered: {:?}",
        legal.abilities
    );

    // The four Swamps are spent, so reading the printed line needs a turn: the
    // untap step is what stands them back up.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four untapped Swamps again, and the Skirge makes no mana of its own"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(id, _)| *id == skirge)
        .expect("with {{2}} in the pool the printed ability is offered");
    assert_eq!(source, skirge, "and it is the Skirge the offer names");

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("the two mana already floating pay for it");

    assert!(
        in_graveyard(&engine, p0, slinking_skirge()).is_some(),
        "\"Sacrifice this creature\" is a cost, paid on announcement \
         (CR 601.2h), so the card is already in its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p0, slinking_skirge()).is_none(),
        "and it is off the battlefield, because the price was the creature"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "the {{2}} came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "drawing a card is no mana ability, so the draw is waiting on the stack"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"Draw a card\": one card left the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "and it is in hand, so an emptied library would not satisfy the count above"
    );
}
