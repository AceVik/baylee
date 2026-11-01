#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
fn wall_of_nets() -> CardIndex {
    card_index("cd112962-b297-403c-bede-c8b587f379b3")
}
#[test]
fn wall_of_nets_blocks_without_permanently_exiling_unlinked_cards() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(SEED, &[wall_of_nets()], &[grizzly_bears()], &[], &[]);
    let wall = lk::obj(&e, p0, wall_of_nets());
    let bears = lk::obj(&e, p1, grizzly_bears());
    assert!(!lk::attackers_offered(&mut e).contains(&wall));
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p1),
    );
    e.apply(
        p1,
        PlayerAction::DeclareAttackers {
            attackers: vec![(bears, Defender::Player(p0))],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    lk::block(&mut e, &[(wall, bears)]);
    lk::through_combat(&mut e);
    assert_eq!(lk::life(&e, p0), 20);
    assert!(on_battlefield(&e, p0, wall_of_nets()).is_some());
    assert!(on_battlefield(&e, p1, grizzly_bears()).is_some());
}
