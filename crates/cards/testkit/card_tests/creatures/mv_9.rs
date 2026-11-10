//! The tests of the cards under `cards/creatures/mv_9/`, one module per card.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

mod iona_shield_of_emeria;

mod fire_dragon;
mod kuro_pitlord;
mod leviathan;
mod reya_dawnbringer;
mod spirit_of_the_night;
mod the_unspeakable;
mod thing_from_the_deep;

/// A duel at seat 0's first main phase: `board` already on its battlefield
/// and `hand` in its hand.
#[track_caller]
fn at_main(board: &[CardIndex], hand: &[CardIndex]) -> Engine<RegistryLookup> {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, board)
        .hand(0, hand)
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, PlayerId::new(0)),
        "seat 0 reaches a main phase of its own"
    );
    engine
}

/// Seat 0 taps everything and casts `card`; the stack settles and the
/// permanent it became is returned.
#[track_caller]
fn cast_and_settle(engine: &mut Engine<RegistryLookup>, card: CardIndex) -> ObjectId {
    let p0 = PlayerId::new(0);
    cast_from_hand(engine, p0, card);
    pass_until(engine, stack_is_empty);
    on_battlefield(engine, p0, card).expect("the spell resolved onto the battlefield")
}

fn life_of(engine: &Engine<RegistryLookup>, seat: usize) -> i32 {
    engine.state().players[seat].life
}
mod crimson_hellkite;
