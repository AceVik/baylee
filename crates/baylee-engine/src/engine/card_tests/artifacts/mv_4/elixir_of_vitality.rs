//! `cards/artifacts/mv_4/elixir_of_vitality.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Elixir of Vitality — {4} artifact: "This artifact enters tapped", "{T},
/// Sacrifice this artifact: You gain 4 life" and "{8}, {T}, Sacrifice this
/// artifact: You gain 8 life".
///
/// Two copies are cast off eight Forests so that every clause is read in one
/// game. The pair arrives **tapped** and offers neither line on the turn it
/// lands, and after the untap step the cheap line costs nothing but the
/// artifact's own tap while the {8} line is only offered once the eight are
/// actually floating in the pool (`can_afford` reads the pool, not the
/// untapped lands). Each activation sacrifices its own Elixir, so 20 life
/// becomes 24 and then 32 only if both printed prices were really paid.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn elixir_of_vitality_enters_tapped_and_sells_itself_for_either_printed_price() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(); 8])
        .hand(0, &[elixir_of_vitality(), elixir_of_vitality()])
        .life(0, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // {4} for each copy, and the eight Forests are the whole board.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        8,
        "eight Forests, eight green"
    );
    cast_with_floating(&mut engine, p0, elixir_of_vitality());
    pass_until(&mut engine, stack_is_empty);
    cast_with_floating(&mut engine, p0, elixir_of_vitality());
    pass_until(&mut engine, stack_is_empty);

    let elixirs = all_on_battlefield(&engine, p0, elixir_of_vitality());
    assert_eq!(elixirs.len(), 2, "both copies resolved onto the table");
    let (cheap, dear) = (elixirs[0], elixirs[1]);
    assert!(
        is_tapped(&engine, cheap) && is_tapped(&engine, dear),
        "\"This artifact enters tapped\" is a real entry and not a placement"
    );
    // Both lines are paid for with the tap symbol, so an artifact that has
    // just entered tapped is offered neither of them.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds a quiet main phase: {:?}", engine.pending())
    };
    assert!(
        !legal
            .abilities
            .iter()
            .any(|(id, _)| *id == cheap || *id == dear),
        "no {{T}} left to pay with, so neither line is offered: {:?}",
        legal.abilities
    );

    // Across the opponent's turn and back: an artifact that entered tapped
    // still untaps in its controller's untap step like anything else.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, cheap) && !is_tapped(&engine, dear),
        "the untap step stood both of them back up"
    );

    // The next turn's eight Forests: the {8} the dear line charges, and no
    // part of what the cheap line charges.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        8,
        "eight Forests again, and the untapped Elixirs make no mana of their own"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(cheap, 0)),
        "its own tap and the artifact is a price any pool pays: {:?}",
        legal.abilities
    );
    assert!(
        legal.abilities.contains(&(dear, 1)),
        "and the {{8}} line is offered now that the eight are floating: {:?}",
        legal.abilities
    );

    // Ability 0: "{T}, Sacrifice this artifact: You gain 4 life."
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: cheap,
                ability_index: 0,
            },
        )
        .expect("the cheap line is payable over an empty pool");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].life, 24, "four life, once");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        8,
        "the cheap line took no mana at all"
    );
    assert!(
        on_battlefield(&engine, p0, elixir_of_vitality()).is_some(),
        "one copy was sacrificed and the other is still standing"
    );

    // Ability 1: "{8}, {T}, Sacrifice this artifact: You gain 8 life."
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: dear,
                ability_index: 1,
            },
        )
        .expect("the eight already floating are the printed {8}");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[0].life,
        32,
        "four and then eight, and no third number"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{8}} came out of the pool"
    );
    assert!(
        on_battlefield(&engine, p0, elixir_of_vitality()).is_none(),
        "each activation sacrifices the artifact that pays it"
    );
    assert_eq!(
        mine(&engine, p0, elixir_of_vitality(), Zone::Graveyard).len(),
        2,
        "and both copies are in their owner's graveyard"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "the life belongs to the seat that paid the price, not to the opponent"
    );
}
