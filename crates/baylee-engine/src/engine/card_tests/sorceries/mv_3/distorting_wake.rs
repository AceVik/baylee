//! Distorting Wake — {X}{U}{U}{U} — Sorcery
//! Oracle: Return X target nonland permanents to their owners' hands.

use super::*;
use baylee_core::generated::index;

#[test]
fn distorting_wake_bounces_x_target_nonland_permanents() {
    let wake = card_index("e5387fc7-c3b3-4933-bffa-580ad793c7ab");
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island(), island()])
        .battlefield(1, &[index::GRIZZLY_BEARS, index::HILL_GIANT, forest()])
        .hand(0, &[wake])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let bears_id = on_battlefield(&engine, p1, index::GRIZZLY_BEARS).unwrap();
    let giant_id = on_battlefield(&engine, p1, index::HILL_GIANT).unwrap();
    let forest_id = on_battlefield(&engine, p1, forest()).unwrap();

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, wake);
    engine.apply(p0, PlayerAction::ChooseNumber(2)).unwrap();

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice for Distorting Wake");
    };
    assert!(options.contains(&bears_id));
    assert!(options.contains(&giant_id));
    assert!(!options.contains(&forest_id));

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears_id, giant_id],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, index::GRIZZLY_BEARS).is_none());
    assert!(on_battlefield(&engine, p1, index::HILL_GIANT).is_none());
    assert!(in_hand(&engine, p1, index::GRIZZLY_BEARS).is_some());
    assert!(in_hand(&engine, p1, index::HILL_GIANT).is_some());
}
