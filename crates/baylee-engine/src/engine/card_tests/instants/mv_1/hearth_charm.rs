//! `cards/instants/mv_1/hearth_charm.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn hearth_charm() -> CardIndex {
    card_index("4ce95005-82b3-4921-b1c4-60d775a012a2")
}

fn iron_heart_chimera() -> CardIndex {
    card_index("fe7c70dd-76ff-4958-9973-6452c7f071ed")
}

/// `Hearth Charm` ({R}, Instant, `Coverage::Implemented`) destroys target
/// artifact creature in mode 0 or makes target creature with power 2 or less
/// unblockable this turn in mode 2.
#[test]
fn hearth_charm_destroys_artifact_creature_or_grants_unblockable() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), grizzly_bears()])
        .battlefield(1, &[iron_heart_chimera()])
        .hand(0, &[hearth_charm(), hearth_charm()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let chimera = on_battlefield(&engine, p1, iron_heart_chimera()).unwrap();
    let bear = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    tap_all_mana(&mut engine, p0);

    // Mode 0: Destroy target artifact creature.
    cast_with_floating(&mut engine, p0, hearth_charm());
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        unreachable!();
    };
    let slot0 = options
        .iter()
        .position(|o| matches!(o.kind, CastModeKind::Mode(0)))
        .unwrap();
    engine.apply(p0, PlayerAction::ChooseMode(slot0)).unwrap();
    aim_at(&mut engine, p0, chimera);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(chimera).unwrap().zone,
        Zone::Graveyard
    );

    // Mode 2: Target creature with power 2 or less can't be blocked this turn.
    cast_with_floating(&mut engine, p0, hearth_charm());
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        unreachable!();
    };
    let slot2 = options
        .iter()
        .position(|o| matches!(o.kind, CastModeKind::Mode(2)))
        .unwrap();
    engine.apply(p0, PlayerAction::ChooseMode(slot2)).unwrap();
    aim_at(&mut engine, p0, bear);
    pass_until(&mut engine, stack_is_empty);
    assert!(keywords(&engine, bear).contains(KeywordSet::UNBLOCKABLE));
}
