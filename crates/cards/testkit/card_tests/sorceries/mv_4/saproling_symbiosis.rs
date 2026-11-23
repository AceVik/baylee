//! `cards/sorceries/mv_4/saproling_symbiosis.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::generated::{index, subtypes};

// oracle_id = "69d22037-fbf7-44d2-81a7-7ce8e32f06ec"
fn saproling_symbiosis() -> CardIndex {
    card_index("69d22037-fbf7-44d2-81a7-7ce8e32f06ec")
}

/// Two controlled creatures produce two Saprolings; the opponent's creature
/// does not enter the count. Paying the optional cost makes this sorcery legal
/// on the opponent's turn without changing that count.
#[test]
fn saproling_symbiosis_with_flash_counts_only_the_casters_creatures() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut board = vec![index::GRIZZLY_BEARS, index::HILL_GIANT];
    board.extend([forest(); 6]);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &board)
        .battlefield(1, &[index::GRAY_OGRE])
        .hand(0, &[saproling_symbiosis()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p1), "p1 reaches its own main");
    engine
        .apply(p1, PlayerAction::PassPriority)
        .expect("p1 gives p0 priority");
    tap_all_mana(&mut engine, p0);

    cast_sorcery_with_optional_flash(&mut engine, p0, saproling_symbiosis());
    pass_until(&mut engine, stack_is_empty);

    let saprolings: Vec<_> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            engine.state().object(*id).is_some_and(|o| {
                o.controller == p0
                    && o.card.is_none()
                    && o.characteristics()
                        .subtypes
                        .contains(subtypes::creature::SAPROLING)
            })
        })
        .collect();
    assert_eq!(saprolings.len(), 2, "one token per controlled creature");
    for token in saprolings {
        assert_eq!(pt(&engine, token), (1, 1));
    }
    assert!(on_battlefield(&engine, p0, index::GRIZZLY_BEARS).is_some());
    assert!(on_battlefield(&engine, p0, index::HILL_GIANT).is_some());
    assert!(on_battlefield(&engine, p1, index::GRAY_OGRE).is_some());
}
