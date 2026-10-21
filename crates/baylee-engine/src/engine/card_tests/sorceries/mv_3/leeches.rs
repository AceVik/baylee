//! Leeches — `{1}{W}{W}` Sorcery (Homelands)
//!
//! Target player loses all poison counters. Leeches deals that much damage to
//! that player.
//!
//! (`Coverage::Partial`: removing poison counters from a player and dealing
//! damage equal to the counters lost are not in `Effect`; casts targeting a
//! player.)

#[allow(clippy::wildcard_imports)]
use super::*;

fn leeches() -> CardIndex {
    card_index("0a265cb5-47c7-4e06-9030-602e17bedae5")
}

#[test]
fn leeches_casts_targeting_player() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[leeches()])
        .battlefield(0, &[plains(), plains(), plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let spell = in_hand(&engine, p0, leeches()).unwrap();
    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: spell })
        .unwrap();
    engine.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, leeches()).is_some());
}
