//! `cards/creatures/mv_7/rampaging_brontodon.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn rampaging_brontodon() -> CardIndex {
    card_index("bfea2229-a1ca-40c1-aaad-5f97add3f079")
}

/// Rampaging Brontodon — a 7/7 trampler for {5}{G}{G}.
#[test]
fn rampaging_brontodon_costs_seven_and_is_a_seven_seven() {
    let mut engine = table7(&lots(forest(), 7), &[rampaging_brontodon()], &[]);
    let dino = cast_and_resolve(&mut engine, rampaging_brontodon());
    assert_body(&engine, dino, (7, 7), KeywordSet::TRAMPLE);
}

/// "Whenever this creature attacks, it gets +1/+1 until end of turn for each
/// land you control": with three Forests it is 10/10 once the trigger has
/// resolved and deals 10; the opponent's lands do not count.
#[test]
fn rampaging_brontodon_grows_per_land_when_it_attacks() {
    let (mut engine, dino, _ogre) = duel_vs_ogre(rampaging_brontodon(), &lots(forest(), 3));
    assert_eq!(pt(&engine, dino), (7, 7), "before attacking");
    declare_attack(&mut engine, &[dino], &[]);
    assert_eq!(pt(&engine, dino), (10, 10), "+3/+3 for three lands");
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(life_of(&engine, 1), 10);
}
