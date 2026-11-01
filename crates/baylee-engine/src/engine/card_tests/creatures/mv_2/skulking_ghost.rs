//! `cards/creatures/mv_2/skulking_ghost.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn skulking_ghost() -> CardIndex {
    card_index("cd97f8b4-e751-4b52-987b-a442aef57809")
}

/// Skulking Ghost (`{1}{B}`, `Coverage::Implemented`): 2/1 flying Spirit that
/// sacrifices itself when it becomes the target of a spell or ability.
#[test]
fn skulking_ghost_sacrifices_itself_when_targeted() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[skulking_ghost(), forest()])
        .hand(0, &[giant_growth()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ghost = on_battlefield(&engine, p0, skulking_ghost()).unwrap();
    assert!(keywords(&engine, ghost).contains(KeywordSet::FLYING));

    cast_from_hand(&mut engine, p0, giant_growth());
    aim_at(&mut engine, p0, ghost);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, skulking_ghost()).is_none());
    assert!(in_graveyard(&engine, p0, skulking_ghost()).is_some());
}

/// "This creature becomes the target": a spell aimed at another creature leaves
/// the Ghost alone.
#[test]
fn skulking_ghost_stays_when_something_else_is_targeted() {
    let mut engine = lk::game(
        SEED,
        &[skulking_ghost(), index::GRIZZLY_BEARS],
        &[],
        &[giant_growth()],
        &[],
    );
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);

    lk::float(&mut engine, P0, &[(ManaColor::Green, 1)]);
    lk::cast_at(&mut engine, P0, giant_growth(), &[bears], &[]);

    assert!(on_battlefield(&engine, P0, skulking_ghost()).is_some());
    assert!(in_graveyard(&engine, P0, skulking_ghost()).is_none());
}

/// "Spell or ability", any controller: an opponent's Lightning Bolt aimed at
/// the Ghost makes its controller sacrifice it.
#[test]
fn skulking_ghost_is_sacrificed_when_an_opponent_targets_it() {
    let mut engine = lk::game(
        SEED,
        &[skulking_ghost()],
        &[],
        &[],
        &[index::LIGHTNING_BOLT],
    );
    let ghost = lk::obj(&engine, P0, skulking_ghost());

    reach_their_main_phase(&mut engine, P1);
    lk::float(&mut engine, P1, &[(ManaColor::Red, 1)]);
    lk::cast_at(&mut engine, P1, index::LIGHTNING_BOLT, &[ghost], &[]);

    assert!(on_battlefield(&engine, P0, skulking_ghost()).is_none());
    assert!(in_graveyard(&engine, P0, skulking_ghost()).is_some());
}
