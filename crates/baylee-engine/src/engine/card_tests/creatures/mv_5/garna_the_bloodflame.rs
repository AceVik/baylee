//! `cards/creatures/mv_5/garna_the_bloodflame.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// "Flash. When Garna enters, return to your hand all creature cards in your
/// graveyard that were put there from anywhere this turn. Other creatures you
/// control have haste." A Steadfast Guard dies on its controller's turn, an
/// Llanowar Elves on the opponent's; Garna, flashed in on the opponent's
/// turn, brings back the Elves and leaves the Guard. Its controller's other
/// creature has haste, Garna and the opponent's creature do not.
#[test]
fn garna_returns_the_creature_cards_put_into_the_graveyard_this_turn() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                steadfast_guard(),
                llanowar_elves(),
                festering_goblin(),
                swamp(),
                mountain(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .battlefield(1, &[steadfast_guard()])
        .hand(0, &[garna_the_bloodflame()])
        .start();
    keep_mulligans(&mut engine);
    let guard = on_battlefield(&engine, p0, steadfast_guard()).unwrap();
    let elves = on_battlefield(&engine, p0, llanowar_elves()).unwrap();
    let goblin = on_battlefield(&engine, p0, festering_goblin()).unwrap();
    let theirs = on_battlefield(&engine, p1, steadfast_guard()).unwrap();
    assert!(!keywords(&engine, goblin).contains(KeywordSet::HASTE));

    reach_main_phase(&mut engine, p0);
    kill(&mut engine, guard);
    reach_their_main_phase(&mut engine, p1);
    kill(&mut engine, elves);
    assert!(in_graveyard(&engine, p0, steadfast_guard()).is_some());
    assert!(in_graveyard(&engine, p0, llanowar_elves()).is_some());

    // Flash: on the opponent's turn.
    cast_from_hand(&mut engine, p0, garna_the_bloodflame());
    pass_until(&mut engine, stack_is_empty);
    let garna = on_battlefield(&engine, p0, garna_the_bloodflame()).expect("resolved");

    assert!(
        in_hand(&engine, p0, llanowar_elves()).is_some(),
        "put into the graveyard this turn"
    );
    assert!(
        in_graveyard(&engine, p0, steadfast_guard()).is_some(),
        "put there last turn: it stays"
    );
    assert!(keywords(&engine, goblin).contains(KeywordSet::HASTE));
    assert!(
        !keywords(&engine, garna).contains(KeywordSet::HASTE),
        "other creatures"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::HASTE),
        "creatures you control"
    );
}

/// "All creature cards": a sorcery put into the graveyard this turn is not
/// one. Giant Growth resolves on the turn Garna enters and stays where it
/// went.
#[test]
fn garna_leaves_a_noncreature_card_put_there_this_turn() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                llanowar_elves(),
                swamp(),
                mountain(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .hand(0, &[giant_growth(), garna_the_bloodflame()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let elves = on_battlefield(&engine, p0, llanowar_elves()).unwrap();
    // Seven mana floats; the Growth takes one and Garna five of the rest.
    cast_from_hand(&mut engine, p0, giant_growth());
    let _ = aim_at(&mut engine, p0, elves);
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, p0, giant_growth()).is_some());
    cast_with_floating(&mut engine, p0, garna_the_bloodflame());
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, garna_the_bloodflame()).is_some());
    assert!(
        in_graveyard(&engine, p0, giant_growth()).is_some(),
        "an instant is no creature card"
    );
}
