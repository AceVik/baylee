//! `cards/sorceries/mv_5/march_of_souls.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn march_of_souls() -> CardIndex {
    card_index("bca7101b-e74b-42af-ac63-1b3defb4137d")
}

/// March of Souls destroys all creatures without allowing regeneration and gives
/// each creature's controller a 1/1 white Spirit token with flying per destroyed creature.
#[test]
fn march_of_souls_destroys_without_regeneration_and_creates_spirits() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                index::GRIZZLY_BEARS,
            ],
        )
        .battlefield(
            1,
            &[index::DRUDGE_SKELETONS, swamp(), index::LLANOWAR_ELVES],
        )
        .hand(0, &[march_of_souls()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, march_of_souls());
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p1),
    );
    tap_all_mana(&mut engine, p1);
    activate(&mut engine, p1, index::DRUDGE_SKELETONS, 0);
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(in_graveyard(&engine, p0, index::GRIZZLY_BEARS).is_some());
    assert!(in_graveyard(&engine, p1, index::DRUDGE_SKELETONS).is_some());
    assert!(in_graveyard(&engine, p1, index::LLANOWAR_ELVES).is_some());

    let p0_creatures: Vec<_> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|&id| {
            let o = engine.state().object(id).unwrap();
            o.controller == p0 && o.characteristics().types.contains(TypeSet::CREATURE)
        })
        .collect();
    let p1_creatures: Vec<_> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|&id| {
            let o = engine.state().object(id).unwrap();
            o.controller == p1 && o.characteristics().types.contains(TypeSet::CREATURE)
        })
        .collect();
    assert_eq!(p0_creatures.len(), 1);
    assert_eq!(p1_creatures.len(), 2);
    assert_eq!(pt(&engine, p0_creatures[0]), (1, 1));
    assert!(
        engine
            .state()
            .object(p0_creatures[0])
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::FLYING)
    );
}
