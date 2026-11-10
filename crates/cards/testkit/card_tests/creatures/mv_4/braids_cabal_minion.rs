//! Oracle scenarios for Braids, Cabal Minion.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn braids_cabal_minion() -> CardIndex {
    card_index("d85aa59b-c6aa-4b2f-885c-59d6e5e6e8b9")
}

#[test]
fn braids_cabal_minion_forces_each_player_to_sacrifice_on_their_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, index::SWAMP)
        .battlefield(0, &[index::SWAMP, index::SWAMP, index::SWAMP, index::SWAMP])
        .battlefield(1, &[index::GRIZZLY_BEARS, index::CRUSADE])
        .hand(0, &[braids_cabal_minion()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);

    cast_from_hand(&mut e, p0, braids_cabal_minion());
    pass_until(&mut e, stack_is_empty);

    let p1_bears = on_battlefield(&e, p1, index::GRIZZLY_BEARS).unwrap();
    let p1_crusade = on_battlefield(&e, p1, index::CRUSADE).unwrap();

    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::ChooseCards { player, .. } if *player == p1),
    );
    let Pending::ChooseCards { options, .. } = e.pending().clone() else {
        panic!("expected sacrifice choice for p1")
    };
    assert!(options.contains(&p1_bears));
    assert!(
        !options.contains(&p1_crusade),
        "enchantment is not an artifact, creature, or land"
    );

    e.apply(
        p1,
        PlayerAction::ChooseObjects {
            objects: vec![p1_bears],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);

    assert!(in_graveyard(&e, p1, index::GRIZZLY_BEARS).is_some());
    assert!(on_battlefield(&e, p1, index::CRUSADE).is_some());
}
