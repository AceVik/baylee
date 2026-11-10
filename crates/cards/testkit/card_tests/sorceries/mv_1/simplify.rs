//! Oracle scenarios for Simplify.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn simplify() -> CardIndex {
    card_index("853cac98-34dc-471f-af87-1a94b0022b67")
}

#[test]
fn simplify_makes_each_player_sacrifice_an_enchantment() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(0, &[index::FOREST, index::CRUSADE])
        .battlefield(1, &[index::CRUSADE])
        .hand(0, &[simplify()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);

    let c0 = on_battlefield(&e, p0, index::CRUSADE).unwrap();
    let c1 = on_battlefield(&e, p1, index::CRUSADE).unwrap();

    cast_from_hand(&mut e, p0, simplify());
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    e.apply(p0, PlayerAction::ChooseObjects { objects: vec![c0] })
        .unwrap();
    e.apply(p1, PlayerAction::ChooseObjects { objects: vec![c1] })
        .unwrap();
    pass_until(&mut e, stack_is_empty);

    assert!(in_graveyard(&e, p0, index::CRUSADE).is_some());
    assert!(in_graveyard(&e, p1, index::CRUSADE).is_some());
}
