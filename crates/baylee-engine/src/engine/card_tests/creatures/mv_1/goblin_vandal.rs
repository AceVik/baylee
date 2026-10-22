//! `cards/creatures/mv_1/goblin_vandal.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GOBLIN_VANDAL, JAYEMDAE_TOME};

/// `Goblin Vandal` ({R}, Creature — Goblin Rogue 1/1) lets you pay `{R}` when
/// it attacks and isn't blocked to destroy target artifact defending player
/// controls and assign no combat damage this turn.
#[test]
fn goblin_vandal_destroys_defenders_artifact_and_assigns_no_combat_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[GOBLIN_VANDAL, mountain()])
        .battlefield(1, &[JAYEMDAE_TOME])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let vandal = on_battlefield(&engine, p0, GOBLIN_VANDAL).unwrap();
    let tome = on_battlefield(&engine, p1, JAYEMDAE_TOME).unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(vandal, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();

    aim_at(&mut engine, p0, tome);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    tap_all_mana(&mut engine, p0);
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.step == crate::turn::Step::Main
    });

    assert!(in_graveyard(&engine, p1, JAYEMDAE_TOME).is_some());
    assert_eq!(engine.state().players[1].life, 20);
}
