//! Oracle scenarios for Tremble.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn tremble_makes_each_player_sacrifice_a_land() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let card = card_index("0fa0f562-e66f-4630-890a-9a4ae24fd6c0");
    let mut e = Duel::new(SEED, index::MOUNTAIN)
        .battlefield(0, &[index::MOUNTAIN, index::MOUNTAIN])
        .battlefield(1, &[index::FOREST])
        .hand(0, &[card])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);

    cast_from_hand(&mut e, p0, card);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. }) || stack_is_empty(e)
    });
    while let Pending::ChooseCards {
        player,
        options,
        min,
        ..
    } = e.pending().clone()
    {
        e.apply(
            player,
            PlayerAction::ChooseObjects {
                objects: options[..usize::from(min)].to_vec(),
            },
        )
        .unwrap();
        pass_until(&mut e, |e| {
            matches!(e.pending(), Pending::ChooseCards { .. }) || stack_is_empty(e)
        });
    }

    assert_eq!(all_on_battlefield(&e, p0, index::MOUNTAIN).len(), 1);
    assert!(all_on_battlefield(&e, p1, index::FOREST).is_empty());
}
