//! `cards/instants/mv_4/memory_deluge.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// "Look at the top X cards of your library, where X is the amount of mana
/// spent to cast this spell. Put two of them into your hand and the rest on
/// the bottom of your library in a random order." — four Islands spent,
/// four cards looked at, two kept, two on the bottom.
#[test]
fn memory_deluge_looks_at_as_many_cards_as_mana_was_spent() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(), island(), island(), island()])
        .hand(0, &[memory_deluge()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, memory_deluge());
    let rest = keep_first(&mut engine, p0, 4, 2);
    let library = engine.state().zones.list(ZoneLocation::Library(p0));
    let mut bottom = library[..2].to_vec();
    bottom.sort_unstable();
    let mut rest = rest;
    rest.sort_unstable();
    assert_eq!(bottom, rest, "the two not kept are the bottom two");
    assert!(in_graveyard(&engine, p0, memory_deluge()).is_some());
}

/// "Flashback {5}{U}{U}": from the graveyard the Deluge costs seven, not its
/// four — four floating is not enough — and seven spent looks at seven.
/// Afterwards it is exiled (CR 702.34a).
#[test]
fn memory_deluge_flashes_back_for_seven_and_looks_at_seven() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(); 11])
        .hand(0, &[memory_deluge()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let islands = lands_of(&engine, p0);
    let first: Vec<ObjectId> = islands[..4].to_vec();
    tap_mana_where(&mut engine, p0, |id| first.contains(&id));
    cast_with_floating(&mut engine, p0, memory_deluge());
    keep_first(&mut engine, p0, 4, 2);
    let deluge = in_graveyard(&engine, p0, memory_deluge()).expect("in the graveyard");

    let second: Vec<ObjectId> = islands[4..8].to_vec();
    tap_mana_where(&mut engine, p0, |id| second.contains(&id));
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&deluge),
        "four floating pays the mana cost, which is not the flashback cost"
    );
    tap_all_mana(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(legal.castable.contains(&deluge), "seven floating pays it");
    engine
        .apply(p0, PlayerAction::CastSpell { card: deluge })
        .unwrap();
    keep_first(&mut engine, p0, 7, 2);
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Exile(p0))
            .contains(&deluge),
        "a flashed-back spell is exiled"
    );
}
