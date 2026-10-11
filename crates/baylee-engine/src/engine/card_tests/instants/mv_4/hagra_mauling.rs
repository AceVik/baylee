//! `cards/instants/mv_4/hagra_mauling.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

#[test]
fn hagra_mauling_destroys_target_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(49, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp()])
        .hand(0, &[hagra_mauling()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("opponent's Elf");

    cast_from_hand(&mut engine, p0, hagra_mauling());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&elf),
        "target creature — the Elf qualifies"
    );
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the Elf is a legal target");

    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_none(),
        "the targeted creature is destroyed"
    );
    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "the destroyed creature is in its owner's graveyard"
    );
    assert!(
        in_graveyard(&engine, p0, hagra_mauling()).is_some(),
        "the resolved spell is in its caster's graveyard"
    );
}

#[test]
fn hagra_reduction_counts_basic_lands_of_each_opponent_not_teammates() {
    use baylee_core::generated::index;
    let p0 = PlayerId::new(0);
    for (multiplayer, teammate, basic, expected) in [
        (false, false, false, true),
        (false, false, true, false),
        (true, false, true, true),
        (true, true, true, false),
    ] {
        let mut duel = Duel::table(SEED, forest(), if multiplayer { 3 } else { 2 })
            .battlefield(0, &[index::GRIZZLY_BEARS])
            .battlefield(1, if basic { &[index::FOREST] } else { &[] })
            .hand(0, &[hagra_mauling()]);
        if teammate {
            duel = duel.team(0, 1).team(1, 2).team(2, 1);
        }
        let mut e = duel.start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        e.dev_state_mut(p0).unwrap().players[0]
            .mana_pool
            .add(ManaColor::Black, 3);
        e.refresh_offer();
        let card = in_hand(&e, p0, hagra_mauling()).unwrap();
        let Pending::Priority { legal, .. } = e.pending() else {
            panic!("priority")
        };
        assert_eq!(legal.castable.contains(&card), expected);
        if expected {
            let bear = on_battlefield(&e, p0, index::GRIZZLY_BEARS).unwrap();
            cast_with_floating(&mut e, p0, hagra_mauling());
            aim_at(&mut e, p0, bear);
            assert_eq!(e.state().players[0].mana_pool.total(), 0);
            pass_until(&mut e, stack_is_empty);
            assert_eq!(e.state().object(bear).unwrap().zone, Zone::Graveyard);
        }
    }
}
