//! `cards/enchantments/mv_3/heat_stroke.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GIANT_TORTOISE, HEAT_STROKE, HILL_GIANT};

/// `Heat Stroke` ({2}{R}, Enchantment, Partial) destroys each creature that
/// blocked this turn at end of combat.
#[test]
fn heat_stroke_destroys_blocking_creature_at_end_of_combat() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[HEAT_STROKE, HILL_GIANT])
        .battlefield(1, &[GIANT_TORTOISE])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let giant = on_battlefield(&engine, p0, HILL_GIANT).unwrap();
    let tortoise = on_battlefield(&engine, p1, GIANT_TORTOISE).unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(giant, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(tortoise, giant)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.step == crate::turn::Step::Main
    });

    assert!(in_graveyard(&engine, p1, GIANT_TORTOISE).is_some());
}
