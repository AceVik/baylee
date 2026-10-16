//! `cards/artifacts/mv_5/chocobo_racetrack.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn chocobo_racetrack() -> CardIndex {
    card_index("7522f7fd-6fda-4ed3-9131-21bf91616a73")
}

/// "Landfall — Whenever a land you control enters, create a 2/2 green Bird
/// creature token with \"Whenever a land you control enters, this token gets
/// +1/+0 until end of turn.\"" The first land makes a 2/2 Bird; a land on a
/// later turn makes another and pumps the first for that turn only.
#[test]
fn a_land_makes_a_bird_and_later_lands_pump_the_birds_already_there() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[chocobo_racetrack()])
        .hand(0, &[forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    assert!(tokens_of(&engine, p0).is_empty());

    play_land(&mut engine, p0, forest());
    pass_until(&mut engine, stack_is_empty);
    let birds = tokens_of(&engine, p0);
    assert_eq!(birds.len(), 1, "one land, one Bird");
    let first = birds[0];
    assert_eq!(pt(&engine, first), (2, 2), "a 2/2");
    let chars = engine.state().object(first).unwrap().characteristics().clone();
    assert!(chars.types.contains(TypeSet::CREATURE));
    assert!(chars.colors.contains(baylee_core::mana::Color::Green));
    assert!(
        chars
            .subtypes
            .contains(baylee_core::generated::subtypes::creature::BIRD)
    );

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && matches!(e.state().turn.phase, Phase::FirstMain)
            && e.state().turn.active == p0
    });
    assert_eq!(pt(&engine, first), (2, 2), "the pump wore off");
    play_land(&mut engine, p0, forest());
    pass_until(&mut engine, stack_is_empty);
    let birds = tokens_of(&engine, p0);
    assert_eq!(birds.len(), 2, "a second Bird");
    let second = *birds.iter().find(|b| **b != first).unwrap();
    assert_eq!(pt(&engine, first), (3, 2), "the old Bird saw the land: +1/+0");
    assert_eq!(pt(&engine, second), (2, 2), "the new one was not there to see it");
}
