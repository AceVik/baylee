//! `cards/sorceries/mv_4/agonizing_memories.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn agonizing_memories() -> CardIndex {
    card_index("2ac9d397-10ca-43ce-bff9-45d631553486")
}

/// `Agonizing Memories` ({2}{B}{B}, Sorcery, `Coverage::Partial`) only looks
/// as written: the caster, not the target, sees the target's whole hand, and
/// nothing is chosen, moved or put on a library (the printed second half).
#[test]
fn agonizing_memories_only_looks_at_the_targets_hand_and_moves_nothing() {
    let mut engine = lk::game(
        SEED,
        &[],
        &[],
        &[agonizing_memories()],
        &[index::GRIZZLY_BEARS, index::HILL_GIANT],
    );
    let bears = in_hand(&engine, P1, index::GRIZZLY_BEARS).unwrap();
    let giant = in_hand(&engine, P1, index::HILL_GIANT).unwrap();
    let library = library_size(&engine, P1);

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 2), (ManaColor::Black, 2)],
    );
    cast_with_floating(&mut engine, P0, agonizing_memories());
    engine.apply(P0, PlayerAction::ChoosePlayer(P1)).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched");
    };
    assert_eq!(player, P0, "the caster is the one who looks");
    assert!(
        options.contains(&bears) && options.contains(&giant),
        "the target's whole hand is shown"
    );
    assert_eq!(
        (min, max),
        (0, 0),
        "as written nothing is chosen from it: the printed two-card choice is not modeled"
    );
    engine
        .apply(P0, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    lk::settle(&mut engine);

    assert!(in_hand(&engine, P1, index::GRIZZLY_BEARS).is_some());
    assert!(in_hand(&engine, P1, index::HILL_GIANT).is_some());
    assert_eq!(library_size(&engine, P1), library, "nothing went on top");
    assert!(in_graveyard(&engine, P0, agonizing_memories()).is_some());
}

/// The look goes to the *caster* even when the caster targets itself: its own
/// hand is the one shown, and the opponent's hand is not.
#[test]
fn agonizing_memories_on_oneself_shows_oneself_and_not_the_opponent() {
    let mut engine = lk::game(
        SEED,
        &[],
        &[],
        &[agonizing_memories(), index::LLANOWAR_ELVES],
        &[index::GRIZZLY_BEARS],
    );
    let elves = in_hand(&engine, P0, index::LLANOWAR_ELVES).unwrap();
    let their_bears = in_hand(&engine, P1, index::GRIZZLY_BEARS).unwrap();

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 2), (ManaColor::Black, 2)],
    );
    cast_with_floating(&mut engine, P0, agonizing_memories());
    engine.apply(P0, PlayerAction::ChoosePlayer(P0)).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        unreachable!("the predicate just matched");
    };
    assert!(options.contains(&elves));
    assert!(!options.contains(&their_bears));
}
