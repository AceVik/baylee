//! `cards/creatures/mv_4/the_fallen.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::turn::Step;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn the_fallen() -> CardIndex {
    card_index("19348c1e-b683-4d2c-b4eb-82e538b314b0")
}

/// The Fallen, already on p0's battlefield, attacks `defender` unblocked in
/// p0's first turn and deals its 2 combat damage.
fn fallen_hits(seats: usize, defender: PlayerId) -> Engine<RegistryLookup> {
    let mut engine = Duel::table(SEED, swamp(), seats)
        .battlefield(0, &[the_fallen()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let fallen = on_battlefield(&engine, P0, the_fallen()).expect("The Fallen is out");
    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::DeclareAttackers
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(fallen, Defender::Player(defender))],
            },
        )
        .expect("The Fallen attacks");
    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::DeclareBlockers
    });
    engine
        .apply(defender, PlayerAction::DeclareBlockers { blockers: vec![] })
        .expect("no blocks");
    pass_until(&mut engine, |e| e.state().turn.step == Step::CombatEnd);
    assert_eq!(
        engine.state().players[defender.get() as usize].life,
        18,
        "two combat damage"
    );
    engine
}

/// The Fallen (19348c1e-b683-4d2c-b4eb-82e538b314b0) — "At the beginning of
/// your upkeep, this creature deals 1 damage to each opponent and each
/// planeswalker it has dealt damage to this game." An opponent it has hit
/// is hit again at each of its controller's upkeeps, and only at those.
///
/// (The planeswalker half is not played here: a planeswalker on a dev board
/// starts with no loyalty.)
#[test]
fn the_fallen_pings_an_opponent_it_has_damaged_at_each_of_your_upkeeps() {
    let mut engine = fallen_hits(2, P1);
    reach_their_main_phase(&mut engine, P1);
    assert_eq!(
        engine.state().players[1].life,
        18,
        "the opponent's own upkeep triggers nothing"
    );
    assert!(walk_to_own_main(&mut engine, P0), "p0's next own main");
    assert_eq!(engine.state().players[1].life, 17, "one ping at my upkeep");
    assert_eq!(
        engine.state().players[0].life,
        20,
        "its controller is never hit"
    );
}

/// An opponent it has never dealt damage to is left alone.
#[test]
fn the_fallen_leaves_an_undamaged_opponent_alone() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[the_fallen()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, P0), "p0 reaches its own main");
    reach_their_main_phase(&mut engine, P1);
    assert!(walk_to_own_main(&mut engine, P0), "p0's next own main");
    assert_eq!(
        engine.state().players[1].life,
        20,
        "never damaged, never pinged"
    );
}

/// At a three-seat table only the opponent it hit is pinged.
#[test]
fn the_fallen_pings_only_the_opponents_it_has_damaged() {
    let mut engine = fallen_hits(3, P1);
    reach_their_main_phase(&mut engine, PlayerId::new(2));
    assert!(walk_to_own_main(&mut engine, P0), "p0's next own main");
    assert_eq!(engine.state().players[1].life, 17, "the damaged opponent");
    assert_eq!(engine.state().players[2].life, 20, "the untouched opponent");
}

/// The damage history belongs to this object: once it has left, nothing
/// pings at the next upkeep.
#[test]
fn the_fallen_stops_pinging_once_it_is_gone() {
    let mut engine = fallen_hits(2, P1);
    let fallen = on_battlefield(&engine, P0, the_fallen()).expect("The Fallen is out");
    kill(&mut engine, fallen);
    reach_their_main_phase(&mut engine, P1);
    assert!(walk_to_own_main(&mut engine, P0), "p0's next own main");
    assert_eq!(
        engine.state().players[1].life,
        18,
        "no ping without the Fallen"
    );
}
