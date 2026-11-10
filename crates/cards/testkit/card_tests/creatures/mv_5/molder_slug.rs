//! Molder Slug makes each player sacrifice an artifact at their upkeep.

use super::*;

fn molder_slug() -> CardIndex {
    card_index("d7c8b636-b7ff-446d-aa51-516e2cfe2730")
}

#[test]
fn molder_slug_hits_each_player_at_that_players_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[molder_slug(), quiet_artifact()])
        .battlefield(1, &[quiet_artifact()])
        .start();
    keep_mulligans(&mut e);
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::ChooseCards { player, .. } if *player == p0),
    );
    let ours = on_battlefield(&e, p0, quiet_artifact()).unwrap();
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![ours],
        },
    )
    .unwrap();
    assert!(in_graveyard(&e, p0, quiet_artifact()).is_some());
    assert!(on_battlefield(&e, p1, quiet_artifact()).is_some());

    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::ChooseCards { player, .. } if *player == p1),
    );
    let theirs = on_battlefield(&e, p1, quiet_artifact()).unwrap();
    e.apply(
        p1,
        PlayerAction::ChooseObjects {
            objects: vec![theirs],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert!(in_graveyard(&e, p1, quiet_artifact()).is_some());
    assert!(in_graveyard(&e, p0, quiet_artifact()).is_some());
}
