//! `cards/sorceries/mv_5/winds_of_rath.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GRIZZLY_BEARS, HILL_GIANT, HOLY_STRENGTH};

fn winds_of_rath() -> CardIndex {
    card_index("a6fd90dc-0ec3-4dce-a77a-4d04f5e254bf")
}

/// `Winds of Rath` ({3}{W}{W}, Sorcery, `Coverage::Implemented`):
/// `Destroy all creatures that aren't enchanted. They can't be regenerated.`
#[test]
fn winds_of_rath_destroys_only_unenchanted_creatures() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                GRIZZLY_BEARS,
            ],
        )
        .battlefield(1, &[HILL_GIANT])
        .hand(0, &[HOLY_STRENGTH, winds_of_rath()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();
    cast_from_hand(&mut engine, p0, HOLY_STRENGTH);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    cast_from_hand(&mut engine, p0, winds_of_rath());
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, GRIZZLY_BEARS).is_some());
    assert!(on_battlefield(&engine, p1, HILL_GIANT).is_none());
    assert!(in_graveyard(&engine, p1, HILL_GIANT).is_some());
}
