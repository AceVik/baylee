//! `cards/enchantments/mv_1/elephant_grass.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn elephant_grass() -> CardIndex {
    card_index("53cc4b93-538d-4193-8bbb-5b73d3ce9d79")
}

fn scathe_zombies() -> CardIndex {
    baylee_core::generated::index::SCATHE_ZOMBIES
}

#[test]
fn elephant_grass_stops_black_creatures_from_attacking_you() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest()])
        .battlefield(1, &[scathe_zombies(), grizzly_bears()])
        .hand(0, &[elephant_grass()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let opp_zombies = on_battlefield(&engine, p1, scathe_zombies()).unwrap();
    cast_from_hand(&mut engine, p0, elephant_grass());
    pass_until(&mut engine, stack_is_empty);

    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending() else {
        unreachable!();
    };
    assert!(!attackers.contains(&opp_zombies));
}
