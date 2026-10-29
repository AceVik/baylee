//! Oracle scenarios for Graceful Antelope.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::{index, subtypes};

fn graceful_antelope() -> CardIndex {
    card_index("8dac693b-c5c5-4d80-9739-a3b95949a213")
}

#[test]
fn graceful_antelope_turns_target_land_into_a_plains_on_combat_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, index::PLAINS)
        .battlefield(0, &[index::PLAINS; 4])
        .battlefield(1, &[index::FOREST])
        .hand(0, &[graceful_antelope()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);

    cast_from_hand(&mut e, p0, graceful_antelope());
    pass_until(&mut e, stack_is_empty);

    let antelope = on_battlefield(&e, p0, graceful_antelope()).unwrap();
    let p1_forest = on_battlefield(&e, p1, index::FOREST).unwrap();
    assert!(keywords(&e, antelope).contains(KeywordSet::PLAINSWALK));

    reach_their_main_phase(&mut e, p1);
    assert!(walk_to_own_main(&mut e, p0));

    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    e.apply(
        p0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(antelope, Defender::Player(p1))],
        },
    )
    .unwrap();
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::ChooseTargets { player, .. } if *player == p0),
    );
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![p1_forest],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    e.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut e, stack_is_empty);

    let chars = e.state().object(p1_forest).unwrap().characteristics();
    assert!(chars.subtypes.contains(subtypes::land::PLAINS));
    assert!(!chars.subtypes.contains(subtypes::land::FOREST));
}
