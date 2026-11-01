//! `cards/sorceries/mv_9/plague_wind.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn plague_wind() -> CardIndex {
    card_index("18ec721f-c1ac-4581-a61d-2f0b09d6bf92")
}

/// Plague Wind ({7}{B}{B} Sorcery):
/// "Destroy all creatures you don't control. They can't be regenerated."
#[test]
fn plague_wind_destroys_only_opponents_creatures_through_regeneration_shields() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let subject = plague_wind();
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                index::GRIZZLY_BEARS,
            ],
        )
        .battlefield(1, &[index::DRUDGE_SKELETONS, swamp()])
        .hand(0, &[subject])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, subject);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p1),
    );
    tap_all_mana(&mut engine, p1);
    activate(&mut engine, p1, index::DRUDGE_SKELETONS, 0);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, index::GRIZZLY_BEARS).is_some());
    assert!(on_battlefield(&engine, p1, index::DRUDGE_SKELETONS).is_none());
    assert!(in_graveyard(&engine, p1, index::DRUDGE_SKELETONS).is_some());
}
