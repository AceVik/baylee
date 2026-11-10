//! `cards/sorceries/mv_3/last_caress.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Last Caress` is a sorcery costing `{2}{B}` under `Coverage::Implemented`.
/// It prints "Target player loses 1 life and you gain 1 life. Draw a card."
///
/// Three clauses name a player and only one of them names a target: the
/// life gain and the draw both belong to "you" (CR 115.10b says the word is no
/// target), so the spell is aimed at the **opponent**, where every clause lands
/// on a different seat and a clause that followed the target instead of the
/// caster shows up in both life totals and both hands. Aimed at the caster, a
/// drain that gave the life and the card to the target would read exactly like
/// the printed one. A target that can only be a player is asked as
/// `Pending::ChoosePlayer`, and the caster is on that list beside the opponent,
/// which is what "target player" means.
#[test]
fn last_caress_targets_player_drains_life_and_draws() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .hand(0, &[last_caress()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let hand_size = |engine: &Engine<RegistryLookup>, seat: PlayerId| {
        engine.state().zones.list(ZoneLocation::Hand(seat)).len()
    };
    let my_hand = hand_size(&engine, p0);
    let their_hand = hand_size(&engine, p1);
    let my_library = library_size(&engine, p0);
    let their_library = library_size(&engine, p1);

    cast_from_hand(&mut engine, p0, last_caress());

    let Pending::ChoosePlayer { player, options } = engine.pending().clone() else {
        panic!(
            "\"target player\" is a player choice, got {:?}",
            engine.pending()
        );
    };
    assert_eq!(player, p0, "the seat that cast it names the target");
    assert!(
        options.contains(&p0) && options.contains(&p1),
        "\"target player\" is either seat, the caster included: {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChoosePlayer(p1))
        .expect("the opponent was one of the players the question enumerated");

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[1].life,
        19,
        "\"target player loses 1 life\": the seat that was named"
    );
    assert_eq!(
        engine.state().players[0].life,
        21,
        "\"and you gain 1 life\": the caster, not the seat that lost"
    );
    assert_eq!(
        library_size(&engine, p0),
        my_library - 1,
        "\"Draw a card\": the caster's library gave up one card"
    );
    assert_eq!(
        hand_size(&engine, p0),
        my_hand,
        "and the caster's hand is back to its size: the sorcery left it and \
         the drawn card came in"
    );
    assert_eq!(
        (library_size(&engine, p1), hand_size(&engine, p1)),
        (their_library, their_hand),
        "the target draws nothing: \"draw a card\" is the caster's"
    );
    assert!(
        in_graveyard(&engine, p0, last_caress()).is_some(),
        "resolved Last Caress sits in its owner's graveyard"
    );
}
