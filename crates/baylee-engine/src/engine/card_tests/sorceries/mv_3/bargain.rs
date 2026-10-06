//! `cards/sorceries/mv_3/bargain.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Bargain is `{2}{W}` for two printed sentences: "Target opponent draws a
/// card" and "You gain 7 life". Both are read off one resolution, because the
/// card is nothing without either — a draw that went to the caster would still
/// show a card moving between zones, and seven is the only life number that
/// tells the printed clause from the one a cantrip would carry. The target
/// question is asserted too: `AnyOpponent` has to decline the caster, which is
/// the half a bare "target player" would lose, and the life has to land on the
/// seat that cast it and not on the seat whose library got shorter.
#[test]
fn bargain_makes_an_opponent_draw_and_its_caster_gain_seven_life() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), plains()])
        .hand(0, &[bargain()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let their_library = library_size(&engine, p1);
    let their_hand = engine.state().zones.list(ZoneLocation::Hand(p1)).len();
    let my_hand = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    cast_from_hand(&mut engine, p0, bargain());
    // "Target opponent" names a seat and no object at all, so the question
    // is `ChoosePlayer` and not the `ChooseTargets` a spell with an object
    // in its sights asks — `TargetSpec::AnyPlayer` is not `AnyTarget`.
    let Pending::ChoosePlayer {
        player,
        options: player_options,
    } = engine.pending().clone()
    else {
        panic!("Bargain targets an opponent, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the caster aims it");
    assert!(
        player_options.contains(&p1),
        "\"target opponent\" reaches the seat across the table: {player_options:?}"
    );
    assert!(
        !player_options.contains(&p0),
        "and never the caster — an opponent is not a target for their own \
         spell: {player_options:?}"
    );
    // The absence is the assertion: the pending carries seats and no object
    // list at all, which is what "target opponent" means and what a spell
    // reaching a permanent would not be.
    assert_eq!(
        player_options.len(),
        1,
        "one opponent at a duel, and no permanent among them: {player_options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChoosePlayer(p1))
        .expect("the seat the question offered is a legal target");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        library_size(&engine, p1),
        their_library - 1,
        "\"target opponent draws a card\" — one card off the top of *their* \
         library, not the caster's"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p1)).len(),
        their_hand + 1,
        "and it is in the opponent's hand"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        my_hand - 1,
        "the caster's hand shrank by the spell it cast and grew by nothing"
    );
    assert_eq!(
        engine.state().players[0].life,
        27,
        "\"You gain 7 life\" — seven, on the seat that cast it"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and none of the life belongs to the opponent who drew"
    );
}
