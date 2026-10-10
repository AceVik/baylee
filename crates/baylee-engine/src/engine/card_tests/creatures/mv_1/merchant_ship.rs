//! `cards/creatures/mv_1/merchant_ship.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn merchant_ship() -> CardIndex {
    card_index("69556f6c-c05b-4902-bac7-012f0ed81b75")
}

fn life_changes(engine: &Engine<RegistryLookup>, seat: PlayerId) -> usize {
    engine
        .journal()
        .entries()
        .iter()
        .filter(|e| matches!(e.event, crate::event::GameEvent::LifeChanged { player, .. } if player == seat))
        .count()
}

fn ship_attacks(blocked: bool) -> Engine<RegistryLookup> {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[merchant_ship(), island()])
        .battlefield(1, &[island(), gray_ogre()])
        .life(0, 10)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let ship = on_battlefield(&engine, p0, merchant_ship()).expect("seated");
    let ogre = on_battlefield(&engine, p1, gray_ogre()).expect("seated");
    declare_band_attack(&mut engine, p0, p1, &[ship]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let blockers = if blocked { vec![(ogre, ship)] } else { vec![] };
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers })
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
    });
    engine
}

/// Unblocked: the trigger fires once and its controller gains exactly 2.
#[test]
fn merchant_ship_gains_two_life_once_when_unblocked() {
    let mut engine = ship_attacks(false);
    assert_eq!(engine.state().players[0].life, 12, "10 + 2");
    assert_eq!(life_changes(&engine, PlayerId::new(0)), 1, "triggered once");
    let _ = &mut engine;
}

/// Blocked: "isn't blocked" is false, so there is no gain.
#[test]
fn merchant_ship_gains_nothing_when_blocked() {
    let engine = ship_attacks(true);
    assert_eq!(engine.state().players[0].life, 10);
    assert_eq!(life_changes(&engine, PlayerId::new(0)), 0);
}
