//! `cards/instants/mv_1/telim_tor_s_edict.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn telim_tor_s_edict() -> CardIndex {
    card_index("0e768d03-b2a3-4783-9cf5-7470f0203349")
}

fn hill_giant() -> CardIndex {
    baylee_core::generated::index::HILL_GIANT
}

/// Telim'Tor's Edict (`{R}`, `Coverage::Implemented`): exiles target permanent
/// you own or control and draws a card at the beginning of the next turn's
/// upkeep.
#[test]
fn telim_tor_s_edict_exiles_your_permanent_and_draws_next_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), grizzly_bears()])
        .battlefield(1, &[hill_giant()])
        .hand(0, &[telim_tor_s_edict()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let giant = on_battlefield(&engine, p1, hill_giant()).unwrap();

    cast_from_hand(&mut engine, p0, telim_tor_s_edict());
    let menu = aim_at(&mut engine, p0, bear);
    assert!(
        !menu.contains(&giant),
        "opponent's permanent that p0 neither owns nor controls cannot be targeted"
    );
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, grizzly_bears()).is_none());
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Exile(p0))
            .contains(&bear)
    );

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1
    );
    // Once only: the next turn of P0's own adds just the draw step's card.
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 2,
        "the Edict's card was drawn once, the draw step's is the second"
    );
}
