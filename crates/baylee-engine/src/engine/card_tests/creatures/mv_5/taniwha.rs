//! `cards/creatures/mv_5/taniwha.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn taniwha() -> CardIndex {
    card_index("371c993a-69b8-4a51-af89-982b79940b2d")
}

/// Taniwha (`{3}{U}{U}`, `Coverage::Implemented`): 7/7 Legendary Serpent with
/// trample, phasing, and an upkeep trigger that phases out all lands you
/// control.
#[test]
fn taniwha_phases_out_and_phases_out_your_lands_on_upkeep_when_phased_in() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(); 5])
        .battlefield(1, &[island(); 2])
        .hand(0, &[taniwha()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let my_islands = lands_of(&engine, p0);
    let their_islands = lands_of(&engine, p1);
    assert_eq!(their_islands.len(), 2);
    cast_from_hand(&mut engine, p0, taniwha());
    pass_until(&mut engine, stack_is_empty);
    let serpent = on_battlefield(&engine, p0, taniwha()).unwrap();
    assert_eq!(pt(&engine, serpent), (7, 7));
    assert!(keywords(&engine, serpent).contains(KeywordSet::TRAMPLE));
    assert!(keywords(&engine, serpent).contains(KeywordSet::PHASING));

    // Next p0 turn: Taniwha phases out during untap, so lands stay phased in.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        engine
            .state()
            .object(serpent)
            .unwrap()
            .status
            .contains(Status::PHASED_OUT)
    );
    assert!(my_islands.iter().all(|&id| {
        !engine
            .state()
            .object(id)
            .unwrap()
            .status
            .contains(Status::PHASED_OUT)
    }));

    // Turn after: Taniwha phases back in during untap, then its upkeep trigger phases out all p0's lands.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !engine
            .state()
            .object(serpent)
            .unwrap()
            .status
            .contains(Status::PHASED_OUT)
    );
    assert!(
        my_islands.iter().all(|&id| engine
            .state()
            .object(id)
            .unwrap()
            .status
            .contains(Status::PHASED_OUT)),
        "all lands controlled by p0 phased out on Taniwha's upkeep trigger"
    );
    assert!(
        their_islands.iter().all(|&id| !engine
            .state()
            .object(id)
            .unwrap()
            .status
            .contains(Status::PHASED_OUT)),
        "only the lands Taniwha's controller controls phase out"
    );
}
