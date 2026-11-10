//! `cards/instants/mv_2/soul_rend.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn soul_rend() -> CardIndex {
    card_index("80d233cc-7748-41a2-bfd6-2189d0136900")
}

fn savannah_lions() -> CardIndex {
    baylee_core::generated::index::SAVANNAH_LIONS
}

/// Soul Rend (`{1}{B}`, `Coverage::Implemented`): destroys target creature if
/// it's white (it can't be regenerated) and draws a card at the beginning of
/// the next turn's upkeep.
#[test]
fn soul_rend_destroys_white_creature_leaves_nonwhite_and_draws_next_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(); 4])
        .battlefield(1, &[savannah_lions(), grizzly_bears()])
        .hand(0, &[soul_rend(), soul_rend()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let lions = on_battlefield(&engine, p1, savannah_lions()).unwrap();
    let bear = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    let swamps = lands_of(&engine, p0);

    // Target a white creature (Savannah Lions): destroyed.
    tap_mana_where(&mut engine, p0, |id| id == swamps[0] || id == swamps[1]);
    cast_with_floating(&mut engine, p0, soul_rend());
    aim_at(&mut engine, p0, lions);
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p1, savannah_lions()).is_none());

    // Target a nonwhite creature (Grizzly Bears): stays on the battlefield.
    tap_mana_where(&mut engine, p0, |id| id == swamps[2] || id == swamps[3]);
    cast_with_floating(&mut engine, p0, soul_rend());
    aim_at(&mut engine, p0, bear);
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p1, grizzly_bears()).is_some());

    // Advance to the next turn's upkeep: both delayed triggers draw a card for p0.
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 2
    );
}
