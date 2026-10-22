//! `cards/creatures/mv_4/goblin_grenadiers.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GOBLIN_GRENADIERS, GRIZZLY_BEARS};

/// `Goblin Grenadiers` ({3}{R}, Creature — Goblin 2/2, Partial) lets you
/// sacrifice it when it attacks and isn't blocked to destroy target creature.
#[test]
fn goblin_grenadiers_sacrifices_on_unblocked_attack_to_destroy_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[GOBLIN_GRENADIERS])
        .battlefield(1, &[GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let grenadiers = on_battlefield(&engine, p0, GOBLIN_GRENADIERS).unwrap();
    let bears = on_battlefield(&engine, p1, GRIZZLY_BEARS).unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(grenadiers, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();

    aim_at(&mut engine, p0, bears);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, GOBLIN_GRENADIERS).is_some());
    assert!(in_graveyard(&engine, p1, GRIZZLY_BEARS).is_some());
}
