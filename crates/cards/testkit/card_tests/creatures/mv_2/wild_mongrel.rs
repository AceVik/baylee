//! Oracle scenarios for Wild Mongrel.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::index;

#[test]
fn wild_mongrel_discards_to_get_plus_1_plus_1_and_become_chosen_color() {
    let p0 = PlayerId::new(0);
    let card = card_index("e9441ab3-7a17-4f9b-a873-e5e82f6e9690");
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(0, &[card])
        .hand(0, &[index::FOREST])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);

    let mongrel = on_battlefield(&e, p0, card).unwrap();
    let forest_hand = in_hand(&e, p0, index::FOREST).unwrap();

    e.apply(
        p0,
        PlayerAction::ActivateAbility {
            source: mongrel,
            ability_index: 0,
        },
    )
    .unwrap();
    if let Pending::ChooseCards { .. } = e.pending().clone() {
        e.apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![forest_hand],
            },
        )
        .unwrap();
    }
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    e.apply(p0, PlayerAction::ChooseMode(1)).unwrap();
    pass_until(&mut e, stack_is_empty);

    assert_eq!(pt(&e, mongrel), (3, 3));
    assert_eq!(
        e.state().object(mongrel).unwrap().characteristics().colors,
        ColorSet::of(Color::Blue)
    );
}
