//! `cards/lands/deserts/desert_of_the_glorified.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Desert of the Glorified prints three sentences and this plays all three,
/// because each one hides the others. It enters tapped, and only a real land
/// drop can show that — `starting_battlefield` places a permanent with
/// `Cause::Setup`, which no replacement effect looks at — so the same turn is
/// used to read the offer a tapped land makes, which is none: the whole price
/// of its `{T}: Add {B}` is the tap it does not have. The second copy stays in
/// hand, because a card can be the land drop or be cycled and not both, and
/// cycling is where `{1}{B}` is really spent out of a pool the land itself
/// helped fill.
#[test]
fn desert_of_the_glorified_enters_tapped_taps_for_black_and_cycles_out_of_hand() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp()])
        .hand(0, &[desert_of_the_glorified(), desert_of_the_glorified()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 reaches its own first main phase"
    );

    let land = play_land(&mut engine, p0, desert_of_the_glorified());
    assert!(
        entered_tapped(&engine, land),
        "\"This land enters tapped\" — read off the permanent, not the card"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "a land drop hands priority back, got {:?}",
            engine.pending()
        )
    };
    assert!(
        !legal.abilities.iter().any(|(source, _)| *source == land),
        "and a permanent that entered tapped offers nothing at all: the whole \
         price of its mana ability is the {{T}} it does not have"
    );

    // Across the opponent's turn and back, because only the untap step stands
    // a land up — until it runs, the {B} is not even in the offer.
    reach_their_main_phase(&mut engine, p1);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "and back around to p0's next turn"
    );
    assert!(!is_tapped(&engine, land), "the untap step ran");

    // Two Swamps and the Desert: three sources and three black. Two would be
    // the lands alone, which is what a land with no mana line of its own
    // would have made.
    tap_all_mana(&mut engine, p0);
    let total = engine.state().players[0].mana_pool.total();
    let black = engine.state().players[0]
        .mana_pool
        .available(ManaColor::Black);
    assert_eq!(total, 3, "two Swamps and the Desert, and nothing else");
    assert_eq!(black, 3, "and the printed {{T}}: Add {{B}} is black");

    // The other copy is still in hand, which is the only zone Cycling lives in.
    let hand_card = in_hand(&engine, p0, desert_of_the_glorified())
        .expect("the second copy has not been played");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(id, _)| *id == hand_card)
        .expect("Cycling is offered from a card in hand, which is what the {1}{B} was tapped for");
    let library_before = library_size(&engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("two of the three black pay {{1}}{{B}}");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, desert_of_the_glorified()).is_some(),
        "discarding the card is half the cost, so it is in the graveyard \
         before anything resolves (CR 601.2h)"
    );
    assert!(
        in_hand(&engine, p0, desert_of_the_glorified()).is_none(),
        "and the card that paid is no longer the card in hand it was activated from"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "Cycling draws a card"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "{{1}}{{B}} out of the three floating — the pool is what says the cost \
         was paid rather than waved"
    );
}
