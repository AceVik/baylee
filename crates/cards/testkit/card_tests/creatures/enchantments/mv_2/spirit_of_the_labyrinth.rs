//! `cards/creatures/enchantments/mv_2/spirit_of_the_labyrinth.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Spirit of the Labyrinth: "Each player can't draw more than one card each
/// turn."
///
/// Played rather than asserted on the modifier, because what a player sees is
/// the second card not arriving. Counsel of the Soratami is the instrument
/// and its whole text is "Draw two cards", so the **partial-carry** half of
/// CR 121.2b is what the hand count says: the spell resolves, one card
/// arrives and the other does not. A one-card draw spell would look the same
/// whether the rule counted draws or refused instructions, and Brainstorm —
/// the first instrument tried — puts two cards back and measures its own
/// rider instead.
///
/// The player on the play skips their first draw step (CR 103.8a), so the
/// turn's allowance is untouched when the spell resolves.
///
/// "Each player" includes the Spirit's own controller, which is the half a
/// card written from Leovold's sentence would get wrong.
#[test]
fn spirit_of_the_labyrinth_lets_a_draw_two_draw_one() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[spirit_of_the_labyrinth(), island(), island(), island()],
        )
        .hand(0, &[counsel_of_the_soratami()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let spirit = on_battlefield(&engine, p0, spirit_of_the_labyrinth())
        .expect("the Spirit is on the battlefield");
    assert_eq!(pt(&engine, spirit), (3, 1));
    assert_eq!(
        engine.state().draw_limit(p0),
        Some(1),
        "the static is on and it names every player, its controller included"
    );
    assert_eq!(
        engine.state().per_turn.draws[0],
        0,
        "the player on the play has drawn nothing yet this turn"
    );

    let before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    cast_from_hand(&mut engine, p0, counsel_of_the_soratami());
    pass_until(&mut engine, stack_is_empty);

    let after = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    assert_eq!(
        after, before,
        "the spell left the hand and one of its two cards came back: \
         CR 121.2b carries the instruction out in part"
    );
    assert_eq!(engine.state().per_turn.draws[0], 1);
}
