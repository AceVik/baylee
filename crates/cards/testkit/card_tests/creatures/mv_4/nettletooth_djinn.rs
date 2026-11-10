//! `cards/creatures/mv_4/nettletooth_djinn.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "f1d300b6-f9cf-40a9-8520-d78cd7d813cf"

/// Nettletooth Djinn — {3}{G} — Creature — Djinn 4/4: "At the beginning of
/// your upkeep, this creature deals 1 damage to you."
///
/// The whole card is the trigger, so nothing short of walking a turn proves
/// anything: the body is read as `(4, 4)` on arrival, and the printed
/// sentence is then read at the only moment it can fire. The opponent's
/// upkeep on the way past is the control — a trigger keyed off "your upkeep"
/// biting every upkeep would have taken p0's life during p1's turn, and a
/// card aimed at "each player" would have taken p1's too. Two of the
/// controller's upkeeps make it a rate and not a one-off, and the Djinn is
/// still standing at the end, so the life belonged to the trigger and not to
/// the creature leaving.
#[test]
fn nettletooth_djinn_bites_its_controller_at_the_beginning_of_its_own_upkeep() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .hand(0, &[nettletooth_djinn()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // {3}{G} out of the four Forests, so the Djinn arrives on a pool that
    // has been spent to nothing and the pool reads no mana afterwards.
    cast_from_hand(&mut engine, p0, nettletooth_djinn());
    pass_until(&mut engine, stack_is_empty);
    let djinn = on_battlefield(&engine, p0, nettletooth_djinn()).expect("the Djinn resolved");
    assert_eq!(pt(&engine, djinn), (4, 4), "the body the card prints");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "all four Forests went into the {{3}}{{G}}"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "the cast happens in a main phase, so no upkeep of p0's has begun yet"
    );

    // Across the opponent's turn: p1's upkeep is not "your upkeep", so the
    // Djinn does not bite on it — the half of the sentence a trigger with a
    // wider step window would get wrong.
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        engine.state().players[0].life,
        20,
        "\"your upkeep\" is the controller's own, and p1's is not p0's"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and the damage sentence names one player, not every player"
    );

    // Back to p0: the triggered ability fired on the way in.
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].life,
        19,
        "\"At the beginning of your upkeep, this creature deals 1 damage to you\""
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "the damage belongs to the seat that controls the Djinn, not to the opponent"
    );
    assert!(
        on_battlefield(&engine, p0, nettletooth_djinn()).is_some(),
        "the Djinn outlives the point of life it took"
    );

    // A second upkeep is a second point: the trigger is a rate and not a
    // once-per-game line, which is what the duration-free sentence means.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].life,
        18,
        "one point per upkeep, so a second turn costs a second point"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and the opponent's life never moved across either turn"
    );
}
