//! Oracle scenarios for Vampiric Dragon.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn vampiric_dragon_pings_creature_and_grows_when_it_dies() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let card = card_index("e8941499-2b31-417b-b2d2-10a144826703");
    let mut e = Duel::new(SEED, index::MOUNTAIN)
        .battlefield(0, &[card, index::MOUNTAIN, index::MOUNTAIN])
        .battlefield(1, &[index::RAGING_GOBLIN])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);

    let dragon = on_battlefield(&e, p0, card).unwrap();
    assert!(keywords(&e, dragon).contains(KeywordSet::FLYING));
    let goblin = on_battlefield(&e, p1, index::RAGING_GOBLIN).unwrap();
    assert_eq!(pt(&e, dragon), (5, 5));

    tap_mana_where(&mut e, p0, |_| true);
    e.apply(
        p0,
        PlayerAction::ActivateAbility {
            source: dragon,
            ability_index: 1,
        },
    )
    .unwrap();
    e.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![goblin],
            players: vec![],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);

    assert!(on_battlefield(&e, p1, index::RAGING_GOBLIN).is_none());
    assert_eq!(pt(&e, dragon), (6, 6));
}
