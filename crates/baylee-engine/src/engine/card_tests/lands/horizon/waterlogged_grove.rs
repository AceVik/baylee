//! `cards/lands/horizon/waterlogged_grove.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Waterlogged Grove prints two lines and this plays both, a turn apart
/// because the first one taps the land for its own price. `{T}, Pay 1 life:
/// Add {G} or {U}` is a mana ability whose whole cost is *not* its own tap,
/// so `tap_all_mana` leaves it standing and both halves of the price are
/// read off the board: the colour question it asks offers exactly green and
/// blue, the mana reaches the pool with nothing on the stack (CR 605.3b),
/// and the life total is one short. The second line — `{1}, {T}, Sacrifice
/// this land: Draw a card` — needs the land upright again, so the turn cycle
/// is load-bearing: it proves the first payment was a tap and not a removal,
/// and it is the only way a `{T}` ability of a land is ever offered again
/// (CR 502.3).
#[test]
fn waterlogged_grove_sells_one_life_for_a_colour_and_itself_for_a_card() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(4711, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[waterlogged_grove()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let grove = play_land(&mut engine, p0, waterlogged_grove());
    assert!(
        !is_tapped(&engine, grove),
        "the card prints no enters-tapped clause, so playing it leaves it upright"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing floats before the land is tapped"
    );
    let life_before = engine.state().players[0].life;

    // Ability 0 is "{T}, Pay 1 life: Add {G} or {U}".
    activate(&mut engine, p0, waterlogged_grove(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{G}} or {{U}}\" is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat names the colour");
    assert!(
        options.contains(&ManaColor::Green) && options.contains(&ManaColor::Blue),
        "the two colours the card prints: {options:?}"
    );
    assert_eq!(options.len(), 2, "and no third: {options:?}");
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .expect("green was one of the colours it offered");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Green),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap and one life");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert_eq!(
        engine.state().players[0].life,
        life_before - 1,
        "the other half of the price was paid"
    );
    assert!(is_tapped(&engine, grove), "and the land paid its own {{T}}");

    // A whole turn cycle, because the second line is the same land's other
    // ability and a tapped land untaps only in its controller's untap step.
    reach_their_main_phase(&mut engine, PlayerId::new(1));
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, grove), "the untap step ran");

    // Mana before the claim (the offer is read off the pool, not off the
    // board), and the Grove is named as the one source kept back so the
    // ability still has an untapped permanent to tap for its cost.
    tap_mana_except(&mut engine, p0, grove);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two Forests, and the Grove was kept standing"
    );
    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    // Ability 1 is "{1}, {T}, Sacrifice this land: Draw a card."
    activate(&mut engine, p0, waterlogged_grove(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, waterlogged_grove()).is_none(),
        "the land sacrificed itself to pay the cost"
    );
    assert!(
        in_graveyard(&engine, p0, waterlogged_grove()).is_some(),
        "and a sacrificed permanent goes to its owner's graveyard"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"draw a card\" is one card off the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "and it is in hand — a draw that emptied the library without \
         filling the hand would satisfy the count above"
    );
}
