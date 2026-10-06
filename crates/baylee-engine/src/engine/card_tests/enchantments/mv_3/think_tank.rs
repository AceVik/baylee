//! `cards/enchantments/mv_3/think_tank.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Think Tank prints one sentence — "At the beginning of your upkeep, surveil
/// 1." — and the card is only itself if the question arrives on its
/// controller's upkeep and the answer moves a card to a graveyard. Both halves
/// are read off one board: the question is an `ArrangePrompt::Surveil` arrangement naming
/// exactly the top card of p0's library with `(0, 1)`, and the answer leaves
/// the library one card shorter with that card in the graveyard and the hand
/// untouched. That last pair is what tells surveil from a scry (the card would
/// have gone to the bottom) and from a draw (the hand would have grown).
#[test]
fn think_tank_surveils_one_on_its_controllers_upkeep() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island()])
        .hand(0, &[think_tank()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, think_tank());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, think_tank()).is_some(),
        "the enchantment resolved onto the battlefield"
    );

    // "Your upkeep" is the word under test, so the walk crosses the opponent's
    // whole turn first: anything asked before p0's next upkeep is a question
    // the printed sentence does not give the card.
    reach_their_main_phase(&mut engine, p1);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Arrange { .. })
    });
    let Pending::Arrange {
        player,
        cards,
        piles,
        prompt,
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p0, "the Think Tank's controller does the looking");
    assert_eq!(
        engine.state().turn.active,
        p0,
        "\"your upkeep\" — the question belongs to the seat that controls it"
    );
    assert_eq!(
        prompt,
        crate::choice::ArrangePrompt::Surveil,
        "surveil is not scry: the card either stays on top or goes to a graveyard"
    );
    assert_eq!(
        piles,
        surveil_piles(1),
        "one card is looked at, and either of the two answers is legal"
    );

    let library_before = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    let top = *library_before.last().expect("p0 has a library");
    assert_eq!(cards, vec![top], "the top card of the library, and only it");
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    engine
        .apply(p0, look_answer(&cards, &[top]))
        .expect("the card the question offered is a legal answer");
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(
        engine
            .state()
            .object(top)
            .expect("the surveilled card is still an object")
            .zone,
        Zone::Graveyard,
        "\"put that card into your graveyard\" — the card that was offered, and not another"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Library(p0)).len(),
        library_before.len() - 1,
        "the surveilled card left the library, so the trigger fired exactly once"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "surveil draws nothing: a hand one longer would be the trigger read as a draw"
    );
    assert!(
        on_battlefield(&engine, p0, think_tank()).is_some(),
        "the enchantment stays where it is, to ask again on the next upkeep"
    );
}
