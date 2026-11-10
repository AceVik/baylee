//! Oracle scenarios for Resilient Wanderer.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn resilient_wanderer() -> CardIndex {
    card_index("cb1e5b14-e440-4ca9-b3f4-e770e158bf69")
}

#[test]
fn resilient_wanderer_discards_to_gain_protection_from_chosen_color() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, index::PLAINS)
        .battlefield(
            0,
            &[
                index::PLAINS,
                index::PLAINS,
                index::PLAINS,
                index::PLAINS,
                index::MOUNTAIN,
                index::GRIZZLY_BEARS,
            ],
        )
        .hand(0, &[resilient_wanderer(), index::FOREST, index::SHOCK])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);

    cast_from_hand(&mut e, p0, resilient_wanderer());
    pass_until(&mut e, stack_is_empty);

    let wanderer = on_battlefield(&e, p0, resilient_wanderer()).unwrap();
    let bear = on_battlefield(&e, p0, index::GRIZZLY_BEARS).unwrap();
    assert!(keywords(&e, wanderer).contains(KeywordSet::FIRST_STRIKE));

    let forest_hand = in_hand(&e, p0, index::FOREST).unwrap();
    e.apply(
        p0,
        PlayerAction::ActivateAbility {
            source: wanderer,
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
        matches!(e.pending(), Pending::ChooseColor { .. })
    });
    e.apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .unwrap();
    pass_until(&mut e, stack_is_empty);

    let mountain_id = on_battlefield(&e, p0, index::MOUNTAIN).unwrap();
    tap_mana_where(&mut e, p0, |id| id == mountain_id);
    cast_with_floating(&mut e, p0, index::SHOCK);
    let menu = aim_at(&mut e, p0, bear);
    assert!(!menu.contains(&wanderer));
}
