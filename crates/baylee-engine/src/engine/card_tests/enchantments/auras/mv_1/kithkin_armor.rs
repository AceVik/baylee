//! `cards/enchantments/auras/mv_1/kithkin_armor.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GRIZZLY_BEARS, HILL_GIANT, KITHKIN_ARMOR};

/// `Kithkin Armor` ({W}, Enchantment — Aura, Partial) prevents enchanted
/// creature from being blocked by creatures with power 3 or greater and can be
/// sacrificed as an activated ability cost.
#[test]
fn kithkin_armor_prevents_high_power_blocks_and_can_be_sacrificed() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[GRIZZLY_BEARS, plains()])
        .battlefield(1, &[HILL_GIANT, llanowar_elves()])
        .hand(0, &[KITHKIN_ARMOR])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();
    let giant = on_battlefield(&engine, p1, HILL_GIANT).unwrap();
    let elves = on_battlefield(&engine, p1, llanowar_elves()).unwrap();

    cast_from_hand(&mut engine, p0, KITHKIN_ARMOR);
    aim_at(&mut engine, p0, bears);
    pass_until(&mut engine, stack_is_empty);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bears, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    assert!(
        engine
            .apply(
                p1,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(giant, bears)],
                },
            )
            .is_err(),
        "3/3 Hill Giant cannot block creature enchanted by Kithkin Armor"
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(elves, bears)],
            },
        )
        .unwrap();

    activate(&mut engine, p0, KITHKIN_ARMOR, 2);
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, KITHKIN_ARMOR).is_some());
}
